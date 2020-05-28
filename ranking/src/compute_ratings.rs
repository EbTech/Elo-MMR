// Copy-paste a spreadsheet column of CF handles as input to this program, then
// paste this program's output into the spreadsheet's ratings column.
use super::read_codeforces::Contest;
use rayon::prelude::*;
use std::cell::{RefCell, RefMut};
use std::cmp::max;
use std::collections::{HashMap, HashSet, VecDeque};

const MU_NEWBIE: f64 = 1500.0; // rating for a new player
const SIG_NEWBIE: f64 = 350.0; // uncertainty for a new player
const SIG_LIMIT: f64 = 100.0; // limiting uncertainty for a player who competed a lot
const SIG_PERF: f64 = 250.0; // variation in individual performances

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

        // Update the variance, avoiding an expensive call to recompute_posterior().
        // Note that we don't update the mode, which may have changed slightly.
        self.approx_posterior.sig *= decay;
    }

    // a fancier but slower substitute for add_noise_uniform(). See paper for details.
    // TODO: optimize using Newton's method.
    fn add_noise_fancy(&mut self, sig_noise: f64) {
        // conveniently update the last rating before applying noise for the next contest
        self.last_rating = self.conservative_rating();

        let decay = 1.0f64.hypot(sig_noise / self.approx_posterior.sig);
        self.approx_posterior.sig *= decay;
        let target = self.approx_posterior.sig.powi(-2);

        let (mut lo, mut hi) = (decay.sqrt(), decay);
        for _ in 0..30 {
            let kappa = (lo + hi) / 2.0;
            let tau_0 = kappa * self.normal_factor.sig;
            let mut test = tau_0.powi(-2);
            for rating in &self.logistic_factors {
                let tau = decay_factor_sig(self.approx_posterior.mu, rating, kappa);
                test += tau.powi(-2);
            }
            if test > target {
                lo = kappa;
            } else {
                hi = kappa;
            }
        }

        let kappa = (lo + hi) / 2.0;
        self.normal_factor.sig *= kappa;
        for rating in &mut self.logistic_factors {
            let tau = decay_factor_sig(self.approx_posterior.mu, rating, kappa);
            rating.sig = tau;
        }
        //println!("{} < {} < {}", decay.sqrt(), kappa, decay);
    }

    fn recompute_posterior(&mut self) {
        let mut sig_inv_sq = self.normal_factor.sig.powi(-2);
        let logistic_vec: Vec<Rating> = self.logistic_factors.iter().cloned().collect();
        let mu = robust_mean(
            &logistic_vec,
            None,
            -self.normal_factor.mu * sig_inv_sq,
            sig_inv_sq,
        );
        for &factor in &self.logistic_factors {
            sig_inv_sq += factor.sig.powi(-2);
        }
        self.approx_posterior = Rating {
            mu,
            sig: sig_inv_sq.recip().sqrt(),
        };
        self.max_rating = max(self.max_rating, self.conservative_rating());
    }

    fn push_performance(&mut self, perf: f64) {
        if self.logistic_factors.len() == 50_000 {
            let logistic = self.logistic_factors.pop_front().unwrap();
            let deviation = self.approx_posterior.mu - logistic.mu;
            let wn = self.normal_factor.sig.powi(-2);
            let wl = (deviation / logistic.sig).tanh() / (deviation * logistic.sig);
            self.normal_factor.mu = (wn * self.normal_factor.mu + wl * logistic.mu) / (wn + wl);
            self.normal_factor.sig = (wn + wl).recip().sqrt();
        }
        self.logistic_factors.push_back(Rating {
            mu: perf,
            sig: SIG_PERF,
        });
    }

    fn conservative_rating(&self) -> i32 {
        (self.approx_posterior.mu - 2.0 * (self.approx_posterior.sig - SIG_LIMIT)).round() as i32
    }
}

fn decay_factor_sig(center: f64, factor: &Rating, kappa: f64) -> f64 {
    let target = ((center - factor.mu) / factor.sig).abs().tanh() / (factor.sig * kappa * kappa);
    let (mut lo, mut hi) = (factor.sig * kappa, factor.sig * kappa * kappa);
    for _ in 0..30 {
        let tau = (lo + hi) / 2.0; 
        let test = ((center - factor.mu) / tau).abs().tanh() / tau;
        if test > target + 1e-12 {
            lo = tau;
        } else {
            hi = tau;
        }
    }
    //println!("{} < {} < {} < {}", factor.sig * kappa, lo, hi, factor.sig * kappa * kappa);
    (lo + hi) / 2.0
}

// Returns something near the mean if the ratings are consistent; near the median if they're far apart.
// offC and offM are constant and slope offsets, respectively. Uses a hybrid of binary search
// (to converge in the worst-case) and Newton's method (for speed in the typical case).
fn robust_mean(all_ratings: &[Rating], extra: Option<usize>, off_c: f64, off_m: f64) -> f64 {
    let mut guess = MU_NEWBIE;
    let (mut lo, mut hi) = (-1000.0, 4500.0);
    loop {
        let mut sum = off_c + off_m * guess;
        let mut sum_prime = off_m;
        for &rating in all_ratings.iter().chain(extra.map(|i| &all_ratings[i])) {
            let incr = ((guess - rating.mu) / rating.sig).tanh() / rating.sig;
            sum += incr;
            sum_prime += rating.sig.powi(-2) - incr * incr
        }
        let next = (guess - sum / sum_prime)
            .max(0.75 * lo + 0.25 * guess)
            .min(0.25 * guess + 0.75 * hi);
        if sum > 0.0 {
            hi = guess;
        } else {
            lo = guess;
        }

        if sum.abs() < 1e-11 {
            return next;
        }
        if hi - lo < 1e-15 {
            println!(
                "WARNING: POSSIBLE FAILURE TO CONVERGE: {}->{} s={} s'={}",
                guess, next, sum, sum_prime
            );
            return next;
        }
        guess = next;
    }
}

// ratings is a list of the participants, ordered from first to last place
// returns: performance of the player in ratings[id] who tied against ratings[lo..hi]
fn compute_performance(
    better: &[Rating],
    worse: &[Rating],
    all: &[Rating],
    extra: Option<usize>,
) -> f64 {
    let pos_offset: f64 = better.iter().map(|rating| rating.sig.recip()).sum();
    let neg_offset: f64 = worse.iter().map(|rating| rating.sig.recip()).sum();
    robust_mean(all, extra, pos_offset - neg_offset, 0.0)
}

pub fn simulate_contest(players: &mut HashMap<String, RefCell<Player>>, contest: &Contest) {
    let sig_noise = ((SIG_LIMIT.powi(-2) - SIG_PERF.powi(-2)).recip() - SIG_LIMIT.powi(2)).sqrt();

    // Make sure the players exist, initializing newcomers with a default rating
    contest.standings.iter().for_each(|&(ref handle, _, _)| {
        players.entry(handle.clone()).or_default();
    });

    // Store guards so that the cells can be released later
    let mut guards: Vec<RefMut<Player>> = contest
        .standings
        .iter()
        .map(|&(ref handle, _, _)| players.get(handle).unwrap().borrow_mut())
        .collect();

    // Get mut references to all requested players, panic if handles are not distinct
    let mut players_ref: Vec<&mut Player> = guards
        .iter_mut()
        .map(std::ops::DerefMut::deref_mut)
        .collect();

    // Update ratings due to waiting period between contests
    let all_ratings: Vec<Rating> = players_ref
        .par_iter_mut()
        .map(|player| {
            player.add_noise_fancy(sig_noise);
            let rating = player.approx_posterior;
            Rating {
                mu: rating.mu,
                sig: rating.sig.hypot(SIG_PERF),
            }
        })
        .collect();

    // The computational bottleneck: update ratings based on contest performance
    players_ref
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, player)| {
            let (_, lo, hi) = contest.standings[i];

            let perf = compute_performance(
                &all_ratings[..lo],
                &all_ratings[hi + 1..],
                &all_ratings,
                Some(i),
            );
            player.push_performance(perf);
            player.recompute_posterior();
            player.last_contest = contest.id;
        });
}

// TODO: does everything below here belong in a separate file?
// Consider refactoring out the write target and the selection of recent contests.

struct RatingData {
    cur_rating: i32,
    max_rating: i32,
    handle: String,
    last_contest: usize,
    last_perf: i32,
    last_delta: i32,
}

pub fn print_ratings(players: &HashMap<String, RefCell<Player>>, contests: &[usize]) {
    const NUM_TITLES: usize = 11;
    const TITLE_BOUND: [i32; NUM_TITLES] = [
        -999, 1000, 1200, 1400, 1600, 1800, 2000, 2200, 2400, 2700, 3000,
    ];
    const TITLE: [&str; NUM_TITLES] = [
        "Ne", "Pu", "Ap", "Sp", "Ex", "CM", "Ma", "IM", "GM", "IG", "LG",
    ];
    const SIX_MONTHS_AGO: usize = 1260;

    use std::io::Write;
    let filename = "../data/CFratings_temp.txt";
    let file = std::fs::File::create(filename).expect("Output file not found");
    let mut out = std::io::BufWriter::new(file);
    let recent_contests: HashSet<usize> = contests
        .iter()
        .copied()
        .skip_while(|&i| i != SIX_MONTHS_AGO)
        .collect();

    let mut sum_ratings = 0.0;
    let mut rating_data = Vec::with_capacity(players.len());
    let mut title_count = vec![0; NUM_TITLES];
    for (handle, player) in players {
        // non-determinism comes from ordering of players
        let player = player.borrow_mut();
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
            if let Some(title_id) = (0..NUM_TITLES)
                .rev()
                .find(|&i| cur_rating >= TITLE_BOUND[i])
            {
                title_count[title_id] += 1;
            }
        }
    }
    rating_data.sort_unstable_by_key(|data| -data.cur_rating);

    writeln!(
        out,
        "Mean rating.mu = {}",
        sum_ratings / players.len() as f64
    )
    .ok();

    for i in (0..NUM_TITLES).rev() {
        writeln!(out, "{} {} x{:6}", TITLE_BOUND[i], TITLE[i], title_count[i]).ok();
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
        writeln!(
            out,
            "perf ={:5}, delta ={:4}",
            data.last_perf, data.last_delta
        )
        .ok();
    }
}
