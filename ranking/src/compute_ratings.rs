// Copy-paste a spreadsheet column of CF handles as input to this program, then
// paste this program's output into the spreadsheet's ratings column.
use rayon::prelude::*;
use std::cmp::max;
use std::collections::{VecDeque, HashSet, HashMap};
use std::fs::File;
use std::io;
use std::str;

const NUM_TITLES: usize = 11;
const TITLE_BOUND: [i32; NUM_TITLES] = [-999,1000,1200,1400,1600,1800,2000,2200,2400,2700,3000];
const TITLE: [&str; NUM_TITLES] = ["Ne","Pu","Ap","Sp","Ex","CM","Ma","IM","GM","IG","LG"];
const MU_NEWBIE: f64 = 1500.0; // rating for a new player
const SIG_NEWBIE: f64 = 350.0; // uncertainty for a new player
const SIG_LIMIT: f64 = 100.0; // limiting uncertainty for a player who competed a lot
const SIG_PERF: f64 = 250.0; // variation in individual performances
const SIX_MONTHS_AGO: usize = 1131;

struct Scanner<R> {
    reader: R,
    buf_str: Vec<u8>,
    buf_iter: str::SplitAsciiWhitespace<'static>,
}

impl<R: io::BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self { reader, buf_str: vec![], buf_iter: "".split_ascii_whitespace() }
    }
    fn token<T: str::FromStr>(&mut self) -> T {
        loop {
            if let Some(token) = self.buf_iter.next() {
                return token.parse().ok().expect("Failed parse");
            }
            self.buf_str.clear();
            self.reader.read_until(b'\n', &mut self.buf_str).expect("Failed read");
            self.buf_iter = unsafe {
                let slice = str::from_utf8_unchecked(&self.buf_str);
                std::mem::transmute(slice.split_ascii_whitespace())
            }
        }
    }
}

fn scanner_from_file(filename: &str) -> Scanner<io::BufReader<std::fs::File>> {
    let file = File::open(filename).expect("Input file not found");
    Scanner::new(io::BufReader::new(file))
}

fn writer_to_file(filename: &str) -> io::BufWriter<std::fs::File> {
    let file = std::fs::File::create(filename).expect("Output file not found");
    io::BufWriter::new(file)
}

pub fn get_contests() -> Vec<usize> {
    let mut team_contests = HashSet::new();
    let mut solo_contests = Vec::new();
    
    let mut scan = scanner_from_file("../data/team_contests.txt");
    for _ in 0..scan.token::<usize>() {
        let contest = scan.token::<usize>();
        team_contests.insert(contest);
    }
    
    scan = scanner_from_file("../data/all_contests.txt");
    for _ in 0..scan.token::<usize>() {
        let contest = scan.token::<usize>();
        if !team_contests.contains(&contest) {
            solo_contests.push(contest);
        }
    }
    
    assert_eq!(team_contests.len(), 17);
    assert_eq!(solo_contests.len(), 948);
    solo_contests
}

fn read_results(contest: usize) -> (String, Vec<(String, usize, usize)>) {
    let filename = format!("../standings/{}.txt", contest);
    let mut scan = scanner_from_file(&filename);
    let num_contestants = scan.token::<usize>();
    let title = scan.buf_iter.by_ref().collect::<Vec<_>>().join(" ");

    let mut seen_handles = HashSet::with_capacity(num_contestants);
    let results: Vec<(String, usize, usize)> = (0..num_contestants).map(|i| {
        let handle = scan.token::<String>();
        let rank_lo = scan.token::<usize>() - 1;
        let rank_hi = scan.token::<usize>() - 1;

        assert!(rank_lo <= i && i <= rank_hi && rank_hi < num_contestants);
        assert!(seen_handles.insert(handle.clone()));
        (handle, rank_lo, rank_hi)
    }).collect();

    (title, results)
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Rating {
    mu: f64,
    sig: f64,
}

impl Default for Rating {
    fn default() -> Self {
        Rating {
            mu: MU_NEWBIE,
            sig: SIG_NEWBIE,
        }
    }
}

#[derive(Default, Clone)]
pub struct Player {
    normal_factor: Rating,
    logistic_factors: VecDeque<Rating>,
    approx_posterior: Rating,
    max_rating: i32,
    last_rating: i32,
    last_contest: usize,
}

impl Player {
    // apply noise to one variable for which we have many estimates
    fn add_noise_uniform(&mut self, sig_noise: f64) {
        // conveniently update the last rating before applying noise for the next contest
        self.last_rating = self.conservative_rating();
        
        // multiply all sigmas by the same decay
        let decay = 1.0f64.hypot(sig_noise / self.approx_posterior.sig);
        self.normal_factor.sig *= decay;
        for rating in &mut self.logistic_factors {
            rating.sig *= decay;
        }
        self.approx_posterior.sig *= decay;
    }
    
    fn recompute_posterior(&mut self) {
        let mut sig_inv_sq = self.normal_factor.sig.powi(-2);
        let logistic_vec = self.logistic_factors.iter().cloned().collect::<Vec<_>>();
        let mu = robust_mean(&logistic_vec, None, -self.normal_factor.mu*sig_inv_sq, sig_inv_sq);
        for &factor in &self.logistic_factors {
            sig_inv_sq += factor.sig.powi(-2);
        }
        self.approx_posterior = Rating{ mu, sig: sig_inv_sq.recip().sqrt() };
    }
    
    fn add_performance(&mut self, perf: f64) {
        if self.logistic_factors.len() == 50_000 {
            let logistic = self.logistic_factors.pop_front().unwrap();
            let wn = self.normal_factor.sig.powi(-2);
            let wl = logistic.sig.powi(-2);
            self.normal_factor.mu = (self.normal_factor.mu * wn + logistic.mu * wl) / (wn + wl);
            self.normal_factor.sig = (wn + wl).recip().sqrt();
        }
        self.logistic_factors.push_back(Rating { mu: perf, sig: SIG_PERF });

        self.recompute_posterior();
        self.max_rating = max(self.max_rating, self.conservative_rating());
    }
    
    fn conservative_rating(&self) -> i32 {
        (self.approx_posterior.mu - 2.0 * (self.approx_posterior.sig - SIG_LIMIT)).round() as i32
    }
}

// Teturns something near the mean if the ratings are consistent; near the median if they're far apart.
// offC and offM are constant and slope offsets, respectively. Uses a hybrid of binary search
// (to converge in the worst-case) and Newton's method (for speed in the typical case).
fn robust_mean(all_ratings: &[Rating], extra: Option<usize>, off_c: f64, off_m: f64) -> f64 {
    let mut guess = MU_NEWBIE;
    let mut max_delta = 1024.0;
    loop {
        let mut sum = off_c + off_m * guess;
        let mut sum_prime = off_m;
        for &rating in all_ratings.iter().chain(extra.map(|i| &all_ratings[i])) {
            let incr = ((guess - rating.mu) / rating.sig).tanh() / rating.sig;
            sum += incr;
            sum_prime += rating.sig.powi(-2) - incr * incr
        }
        let decr = (sum / sum_prime).max(-max_delta).min(max_delta);
        guess -= decr;
        if sum.abs() < 1e-12 {
            return guess;
        }

        max_delta *= 0.75;
        if max_delta < 1e-9 {
            println!("SLOW CONVERGENCE: g={} s={} s'={} d={}", guess, sum, sum_prime, decr);
        }
    }
}

// ratings is a list of the participants, ordered from first to last place
// returns: performance of the player in ratings[id] who tied against ratings[lo..hi]
fn performance(better: &[Rating], worse: &[Rating], all: &[Rating], extra: Option<usize>) -> f64 {
    let pos_offset: f64 = better.iter().map(|rating| rating.sig.recip()).sum();
    let neg_offset: f64 = worse.iter().map(|rating| rating.sig.recip()).sum();
    robust_mean(all, extra, pos_offset - neg_offset, 0.0)
}

pub fn get_players_by_ref_mut<'a>(players: &'a mut HashMap<String, Player>,
                                  results: &[(String,usize,usize)]) -> Vec<&'a mut Player> {
    // Make sure the players exist, initializing with a default rating if necessary.
    results.iter().for_each(|&(ref handle, _, _)| {
        players.entry(handle.clone()).or_default();
    });

    // Produce mut references to all the requested players. The handles MUST be distinct.
    results.iter().map(|&(ref handle, _, _)| {
        let player = players.get_mut(handle).unwrap() as *mut _;
        unsafe { &mut *player }
    }).collect()
}

pub fn simulate_contest(players: &mut HashMap<String, Player>, contest: usize) {
    let sig_noise = ( (SIG_LIMIT.powi(-2) - SIG_PERF.powi(-2)).recip() - SIG_LIMIT.powi(2) ).sqrt();
    
    let (title, results) = read_results(contest);
    println!("Processing {} contestants in contest/{}: {}", results.len(), contest, title);

    let mut players_ref = get_players_by_ref_mut(players, &results);

    let all_ratings: Vec<Rating> = players_ref.par_iter_mut().map(|player| {
        player.add_noise_uniform(sig_noise);
        let rating = player.approx_posterior;
        Rating { mu: rating.mu, sig: rating.sig.hypot(SIG_PERF)  }
    }).collect();
    
    // The computational bottleneck is the ratings updates here
    players_ref.par_iter_mut().enumerate().for_each(|(i, player)| {
        let (_, lo, hi) = results[i];
        
        let perf = performance(&all_ratings[..lo],
                               &all_ratings[hi+1..],
                               &all_ratings, Some(i));
        player.add_performance(perf);
        player.last_contest = contest;
    });
}

struct RatingData {
    cur_rating: i32,
    max_rating: i32,
    handle: String,
    last_contest: usize,
    last_perf: i32,
    last_delta: i32,
}

pub fn print_ratings(players: &HashMap<String, Player>) {
    use io::Write;
    let mut out = writer_to_file("../data/CFratings_temp.txt");
    let recent_contests: HashSet<usize> = get_contests().into_iter()
                         .skip_while(|&i| i != SIX_MONTHS_AGO).collect();
    
    let mut sum_ratings = 0.0;
    let mut rating_data = Vec::with_capacity(players.len());
    let mut title_count = vec![0; NUM_TITLES];
    for (handle, player) in players { // non-determinism comes from ordering of players
        sum_ratings += player.approx_posterior.mu;
        let cur_rating = player.conservative_rating();
        let max_rating = player.max_rating;
        let handle = handle.clone();
        let last_contest = player.last_contest;
        let last_perf = player.logistic_factors.back().unwrap().mu.round() as i32;
        let last_delta = cur_rating - player.last_rating;
        rating_data.push(RatingData {
            cur_rating,
            max_rating,
            handle,
            last_contest,
            last_perf,
            last_delta,
        });
        
        if recent_contests.contains(&last_contest) {
            if let Some(title_id) = (0..NUM_TITLES).rev().find(|&i| cur_rating >= TITLE_BOUND[i]) {
                title_count[title_id] += 1;
            }
        }
    }
    rating_data.sort_unstable_by_key(|data| -data.cur_rating);
    
    writeln!(out, "Mean rating.mu = {}", sum_ratings / players.len() as f64).ok();
    
    for i in (0..NUM_TITLES).rev() {
        writeln!(out, "{} {} x {}", TITLE_BOUND[i], TITLE[i], title_count[i]).ok();
    }
    
    let mut rank = 0;
    for data in rating_data {
        if recent_contests.contains(&data.last_contest) {
            rank += 1;
            write!(out, "{:6}", rank).ok();
        } else {
            write!(out, "{:>6}", "-").ok();
        }
        write!(out, " {:4}({:4})", data.cur_rating, data.max_rating).ok();
        write!(out, " {:<26}contest/{:4}: ", data.handle, data.last_contest).ok();
        writeln!(out, "perf ={:5}, delta ={:4}", data.last_perf, data.last_delta).ok();
    }
}
