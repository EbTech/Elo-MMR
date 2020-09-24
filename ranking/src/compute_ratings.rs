// Copy-paste a spreadsheet column of CF handles as input to this program, then
// paste this program's output into the spreadsheet's ratings column.
use super::contest_config::Contest;
use std::cell::{RefCell, RefMut};
use std::cmp::max;
use std::collections::{HashMap, VecDeque};

pub const MU_NEWBIE: f64 = 1500.0; // rating for a new player
pub const SIG_NEWBIE: f64 = 350.0; // uncertainty for a new player
pub const MAX_HISTORY_LEN: usize = 500; // maximum number of recent performances to keep

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rating {
    pub mu: f64,
    pub sig: f64,
}

#[derive(Clone)]
pub struct Player {
    pub name: String,
    pub normal_factor: Rating,
    pub logistic_factors: VecDeque<Rating>,
    pub approx_posterior: Rating,
    pub num_contests: usize,
    pub max_rating: i32,
    pub last_rating: i32,
    pub last_contest: usize,
    pub last_contest_time: u64,
}

impl Player {
    pub fn with_rating(name: String, mu: f64, sig: f64) -> Self {
        Player {
            name: name,
            normal_factor: Rating { mu, sig },
            logistic_factors: VecDeque::new(),
            approx_posterior: Rating { mu, sig },
            num_contests: 0,
            max_rating: 0,
            last_rating: 0,
            last_contest: 0,
            last_contest_time: 0,
        }
    }

    // the simplest noising method, in which the rating with uncertainty is a Markov state
    pub fn add_noise_and_collapse(&mut self, sig_noise: f64) {
        self.approx_posterior.sig = self.approx_posterior.sig.hypot(sig_noise);
        self.normal_factor = self.approx_posterior;
        self.logistic_factors.clear();
    }

    // apply noise to one variable for which we have many estimates
    #[allow(dead_code)]
    pub fn add_noise_uniform(&mut self, sig_noise: f64) {
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
    #[allow(dead_code)]
    pub fn add_noise_fancy(&mut self, sig_noise: f64) {
        let decay = 1.0f64.hypot(sig_noise / self.approx_posterior.sig);
        self.approx_posterior.sig *= decay;
        let target = self.approx_posterior.sig.powi(-2);

        let (mut lo, mut hi) = (decay.sqrt(), decay);
        for _ in 0..30 {
            let kappa = (lo + hi) / 2.;
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

        let kappa = (lo + hi) / 2.;
        self.normal_factor.sig *= kappa;
        for rating in &mut self.logistic_factors {
            let tau = decay_factor_sig(self.approx_posterior.mu, rating, kappa);
            rating.sig = tau;
        }
        //println!("{} < {} < {}", decay.sqrt(), kappa, decay);
    }

    pub fn recompute_posterior(&mut self) {
        let mut sig_inv_sq = self.normal_factor.sig.powi(-2);
        let mu = robust_average(
            self.logistic_factors.iter().cloned(),
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

    pub fn push_performance(&mut self, rating: Rating) {
        if self.logistic_factors.len() == MAX_HISTORY_LEN {
            let logistic = self.logistic_factors.pop_front().unwrap();
            let deviation = self.approx_posterior.mu - logistic.mu;
            let wn = self.normal_factor.sig.powi(-2);
            let wl = (deviation / logistic.sig).tanh() / (deviation * logistic.sig);
            //let wl_as_normal = logistic.sig.powi(-2);
            self.normal_factor.mu = (wn * self.normal_factor.mu + wl * logistic.mu) / (wn + wl);
            self.normal_factor.sig = (wn + wl).recip().sqrt();
        }
        self.logistic_factors.push_back(rating);
    }

    pub fn conservative_rating(&self) -> i32 {
        // TODO: erase magic number 100.
        (self.approx_posterior.mu - 2. * (self.approx_posterior.sig - 100.)).round() as i32
    }
}

#[allow(dead_code)]
fn decay_factor_sig(center: f64, factor: &Rating, kappa: f64) -> f64 {
    let deviation = (center - factor.mu).abs();
    let target = (deviation / factor.sig).tanh() / (factor.sig * kappa * kappa);
    let (mut lo, mut hi) = (factor.sig * kappa / 2., factor.sig * kappa * kappa * 2.);
    let mut guess = (lo + hi) / 2.;
    loop {
        let tanh_factor = (deviation / guess).tanh();
        let test = tanh_factor / guess;
        let test_prime =
            ((tanh_factor * tanh_factor - 1.) * deviation / guess - tanh_factor) / (guess * guess);
        let test_error = test - target;
        let next = (guess - test_error / test_prime)
            .max(0.75 * lo + 0.25 * guess)
            .min(0.25 * guess + 0.75 * hi);
        if test_error * factor.sig * factor.sig > 0. {
            lo = guess;
        } else {
            hi = guess;
        }

        if test_error.abs() * factor.sig * factor.sig < 1e-11 {
            //println!("{} < {} < {}", factor.sig * kappa, guess, factor.sig * kappa * kappa);
            return next;
        }
        if hi - lo < 1e-15 * factor.sig {
            println!(
                "WARNING: POSSIBLE FAILURE TO CONVERGE: {}->{} e={} e'={}",
                guess, next, test_error, test_prime
            );
            return next;
        }
        guess = next;
    }
}

// Returns the unique zero of the following strictly increasing function of x:
// offset + slope * x + sum_i tanh((x-mu_i)/sig_i) / sig_i
// We must have slope != 0 or |offset| < sum_i 1/sig_i in order for the zero to exist.
// If offset == slope == 0, we get a robust weighted average of the mu_i's. Uses hybrid of
// binary search (to converge in the worst-case) and Newton's method (for speed in the typical case).
pub fn robust_average(
    all_ratings: impl Iterator<Item = Rating> + Clone,
    offset: f64,
    slope: f64,
) -> f64 {
    let (mut lo, mut hi) = (-1000.0, 4500.0);
    let mut guess = MU_NEWBIE;
    loop {
        let mut sum = offset + slope * guess;
        let mut sum_prime = slope;
        for rating in all_ratings.clone() {
            let z = (guess - rating.mu) / rating.sig;
            let incr = z.tanh() / rating.sig;
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

pub trait RatingSystem {
    fn win_probability(&self, player: &Rating, foe: &Rating) -> f64;
    fn round_update(&mut self, standings: Vec<(&mut Player, usize, usize)>);
}

fn update_player_metadata(player: &mut Player, contest: &Contest) {
    player.num_contests += 1;
    player.last_contest = contest.id;
    assert!(player.last_contest_time <= contest.time_seconds);
    player.last_contest_time = contest.time_seconds;
    player.last_rating = player.conservative_rating();
}

pub fn simulate_contest(
    players: &mut HashMap<String, RefCell<Player>>,
    contest: &Contest,
    system: &mut dyn RatingSystem,
) {
    // If a player is competing for the first time, initialize with a default rating
    contest.standings.iter().for_each(|&(ref handle, _, _)| {
        players.entry(handle.clone()).or_insert_with(|| {
            RefCell::new(Player::with_rating(handle.clone(), MU_NEWBIE, SIG_NEWBIE))
        });
    });

    // Low-level magic: verify that handles are distinct and store guards so that the cells
    // can be released later. This setup enables safe parallel processing.
    let mut guards: Vec<RefMut<Player>> = contest
        .standings
        .iter()
        .map(|&(ref handle, _, _)| players.get(handle).expect("Duplicate handles").borrow_mut())
        .collect();

    // Update player metadata and get &mut references to all requested players
    let standings: Vec<(&mut Player, usize, usize)> = guards
        .iter_mut()
        .map(|player| {
            update_player_metadata(player, contest);
            std::ops::DerefMut::deref_mut(player)
        })
        .zip(contest.standings.clone())
        .map(|(player, (_, lo, hi))| (player, lo, hi))
        .collect();

    system.round_update(standings);
}

// TODO: does everything below here belong in a separate file?
// Consider refactoring out the write target and the selection of recent contests.

struct RatingData {
    cur_rating: i32,
    max_rating: i32,
    handle: String,
    last_contest: usize,
    last_contest_time: u64,
    last_perf: i32,
    last_delta: i32,
}

pub fn print_ratings(players: &HashMap<String, RefCell<Player>>, rated_since: u64) {
    const NUM_TITLES: usize = 11;
    const TITLE_BOUND: [i32; NUM_TITLES] = [
        -999, 1000, 1200, 1400, 1600, 1800, 2000, 2200, 2400, 2700, 3000,
    ];
    const TITLE: [&str; NUM_TITLES] = [
        "Ne", "Pu", "Ap", "Sp", "Ex", "CM", "Ma", "IM", "GM", "IG", "LG",
    ];

    use std::io::Write;
    let filename = "../data/CFratings_temp.txt";
    let file = std::fs::File::create(filename).expect("Output file not found");
    let mut out = std::io::BufWriter::new(file);

    let mut rating_data = Vec::with_capacity(players.len());
    let mut title_count = vec![0; NUM_TITLES];
    let sum_ratings = {
        let mut ratings: Vec<f64> = players
            .iter()
            .map(|(_, player)| player.borrow().approx_posterior.mu)
            .collect();
        ratings.sort_by(|a, b| a.partial_cmp(b).expect("NaN is unordered"));
        ratings.into_iter().sum::<f64>()
    };
    for (handle, player) in players {
        let player = player.borrow_mut();
        let cur_rating = player.conservative_rating();
        let max_rating = player.max_rating;
        let handle = handle.clone();
        let last_contest = player.last_contest;
        let last_contest_time = player.last_contest_time;
        let last_perf = player
            .logistic_factors
            .back()
            .map(|r| r.mu.round() as i32)
            .unwrap_or(0);
        let last_delta = cur_rating - player.last_rating;
        rating_data.push(RatingData {
            cur_rating,
            max_rating,
            handle,
            last_contest,
            last_contest_time,
            last_perf,
            last_delta,
        });

        if last_contest_time > rated_since {
            if let Some(title_id) = (0..NUM_TITLES)
                .rev()
                .find(|&i| cur_rating >= TITLE_BOUND[i])
            {
                title_count[title_id] += 1;
            }
        }
    }
    rating_data.sort_unstable_by_key(|data| (-data.cur_rating, data.handle.clone()));

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
        if data.last_contest_time > rated_since {
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
