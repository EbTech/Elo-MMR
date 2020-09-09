// Copy-paste a spreadsheet column of CF handles as input to this program, then
// paste this program's output into the spreadsheet's ratings column.
use super::contest_config::Contest;
use rayon::prelude::*;
use std::cell::{RefCell, RefMut};
use std::cmp::max;
use std::collections::{HashMap, VecDeque};

const MU_NEWBIE: f64 = 1500.0; // rating for a new player
const SIG_NEWBIE: f64 = 350.0; // uncertainty for a new player
const MAX_HISTORY_LEN: usize = 500; // maximum number of recent performances to keep

#[derive(Clone, Copy, PartialEq, Debug)]
struct Rating {
    mu: f64,
    sig: f64,
}

#[derive(Clone)]
pub struct Player {
    normal_factor: Rating,
    logistic_factors: VecDeque<Rating>,
    approx_posterior: Rating,
    num_contests: usize,
    max_rating: i32,
    last_rating: i32,
    last_contest: usize,
    last_contest_time: u64,
}

impl Player {
    fn with_rating(mu: f64, sig: f64) -> Self {
        Player {
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
    fn add_noise_and_collapse(&mut self, sig_noise: f64) {
        self.approx_posterior.sig = self.approx_posterior.sig.hypot(sig_noise);
        self.normal_factor = self.approx_posterior;
        self.logistic_factors.clear();
    }

    // apply noise to one variable for which we have many estimates
    fn add_noise_uniform(&mut self, sig_noise: f64) {
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

    fn recompute_posterior(&mut self) {
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

    fn push_performance(&mut self, rating: Rating) {
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

    fn conservative_rating(&self) -> i32 {
        // TODO: erase magic number 100.
        (self.approx_posterior.mu - 2. * (self.approx_posterior.sig - 100.)).round() as i32
    }
}

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
fn robust_average(
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
    better: impl Iterator<Item = Rating> + Clone,
    tied: impl Iterator<Item = Rating> + Clone,
    worse: impl Iterator<Item = Rating> + Clone,
) -> f64 {
    let all = better.clone().chain(tied).chain(worse.clone());
    let pos_offset: f64 = better.map(|rating| rating.sig.recip()).sum();
    let neg_offset: f64 = worse.map(|rating| rating.sig.recip()).sum();
    robust_average(all, pos_offset - neg_offset, 0.)
}

// ratings is a list of the participants, ordered from first to last place
// returns: performance of the player in ratings[id] who tied against ratings[lo..hi]
fn rate_performance_geo(
    better: &[Rating],
    worse: &[Rating],
    all: &[Rating],
    my_rating: Rating,
) -> f64 {
    // The conversion is 2*rank - 1/my_sig = 2*pos_offset + tied_offset = pos - neg + all
    let pos_offset: f64 = better.iter().map(|rating| rating.sig.recip()).sum();
    let neg_offset: f64 = worse.iter().map(|rating| rating.sig.recip()).sum();
    let all_offset: f64 = all.iter().map(|rating| rating.sig.recip()).sum();

    let ac_rank = 0.5 * (pos_offset - neg_offset + all_offset + my_rating.sig.recip());
    let ex_rank = 0.5
        * (my_rating.sig.recip()
            + all
                .iter()
                .map(|rating| (1. + ((rating.mu - my_rating.mu) / rating.sig).tanh()) / rating.sig)
                .sum::<f64>());

    let geo_rank = (ac_rank * ex_rank).sqrt();
    let geo_offset = 2. * geo_rank - my_rating.sig.recip() - all_offset;
    let geo_rating = robust_average(all.iter().cloned(), geo_offset, 0.);
    0.5 * (my_rating.mu + geo_rating)
}

pub trait RatingSystem {
    fn round_update(&self, standings: Vec<(&mut Player, usize, usize)>);
}

/// Elo-R system details: https://github.com/EbTech/EloR/blob/master/paper/EloR.pdf
pub struct EloRSystem {
    sig_perf: f64,  // variation in individual performances
    sig_limit: f64, // limiting uncertainty for a player who competed a lot
}

impl Default for EloRSystem {
    fn default() -> Self {
        Self {
            sig_perf: 250.,
            sig_limit: 100.,
        }
    }
}

impl RatingSystem for EloRSystem {
    fn round_update(&self, mut standings: Vec<(&mut Player, usize, usize)>) {
        let sig_noise = ((self.sig_limit.powi(-2) - self.sig_perf.powi(-2)).recip()
            - self.sig_limit.powi(2))
        .sqrt();

        // Update ratings due to waiting period between contests
        let all_ratings: Vec<Rating> = standings
            .par_iter_mut()
            .map(|(player, _, _)| {
                player.add_noise_and_collapse(sig_noise);
                let rating = player.approx_posterior;
                Rating {
                    mu: rating.mu,
                    sig: rating.sig.hypot(self.sig_perf),
                }
            })
            .collect();

        // The computational bottleneck: update ratings based on contest performance
        standings
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (player, lo, hi))| {
                let perf = compute_performance(
                    all_ratings[..lo].iter().cloned(),
                    all_ratings[lo..=hi]
                        .iter()
                        .cloned()
                        .chain(std::iter::once(all_ratings[i])),
                    all_ratings[hi + 1..].iter().cloned(),
                );
                player.push_performance(Rating {
                    mu: perf,
                    sig: self.sig_perf,
                });
                player.recompute_posterior();
            });
    }
}

/// Codeforces system details: https://codeforces.com/blog/entry/20762
pub struct CodeforcesSystem {
    sig_perf: f64,
}

impl Default for CodeforcesSystem {
    fn default() -> Self {
        Self {
            sig_perf: 800. / 10f64.ln(),
        }
    }
}

impl RatingSystem for CodeforcesSystem {
    fn round_update(&self, standings: Vec<(&mut Player, usize, usize)>) {
        let all_ratings: Vec<Rating> = standings
            .par_iter()
            .map(|(player, _, _)| Rating {
                mu: player.approx_posterior.mu,
                sig: self.sig_perf,
            })
            .collect();

        standings
            .into_par_iter()
            .zip(all_ratings.par_iter())
            .for_each(|((player, lo, hi), &my_rating)| {
                player.approx_posterior.mu = rate_performance_geo(
                    &all_ratings[..lo],
                    &all_ratings[hi + 1..],
                    &all_ratings,
                    my_rating,
                );
            });
    }
}

/// TopCoder system details: https://www.topcoder.com/community/competitive-programming/how-to-compete/ratings
pub struct TopCoderSystem {}

impl Default for TopCoderSystem {
    fn default() -> Self {
        Self {}
    }
}

impl RatingSystem for TopCoderSystem {
    fn round_update(&self, standings: Vec<(&mut Player, usize, usize)>) {
        use statrs::distribution::{InverseCDF, Normal};
        use statrs::function::erf::erf;
        let standard_normal = Normal::new(0.0, 1.0).unwrap();

        let num_coders = standings.len() as f64;
        let ave_rating = standings
            .iter()
            .map(|&(ref player, _, _)| player.approx_posterior.mu)
            .sum::<f64>()
            / num_coders;

        let c_factor = {
            let mut mean_vol_sq = standings
                .iter()
                .map(|&(ref player, _, _)| player.approx_posterior.sig.powi(2))
                .sum::<f64>()
                / num_coders;
            if num_coders > 1. {
                mean_vol_sq += standings
                    .iter()
                    .map(|&(ref player, _, _)| (player.approx_posterior.mu - ave_rating).powi(2))
                    .sum::<f64>()
                    / (num_coders - 1.);
            }
            mean_vol_sq.sqrt()
        };

        let new_ratings: Vec<Rating> = standings
            .par_iter()
            .map(|(player, lo, hi)| {
                let old_rating = player.approx_posterior.mu;
                let vol_sq = player.approx_posterior.sig.powi(2);
                let win_pr = |rating: &Rating| {
                    0.5 * (1.
                        + erf(
                            (rating.mu - old_rating) / (2. * (rating.sig.powi(2) + vol_sq)).sqrt()
                        ))
                };

                let ex_rank = standings
                    .iter()
                    .map(|&(ref foe, _, _)| (win_pr(&foe.approx_posterior)))
                    .sum::<f64>();
                let ac_rank = 0.5 * (1 + lo + hi) as f64;

                let ex_perf = -standard_normal.inverse_cdf(ex_rank / num_coders);
                let ac_perf = -standard_normal.inverse_cdf(ac_rank / num_coders);

                let perf_as = old_rating + c_factor * (ac_perf - ex_perf);

                let weight = 1. / (1. - (0.42 / (player.num_contests + 1) as f64 + 0.18)) - 1.;

                let cap = 150. + 1500. / (player.num_contests + 2) as f64;

                let try_rating = (old_rating + weight * perf_as) / (1. + weight);
                let new_rating = try_rating.max(old_rating - cap).min(old_rating + cap);

                let new_vol =
                    ((new_rating - old_rating).powi(2) / weight + vol_sq / (1. + weight)).sqrt();

                Rating {
                    mu: new_rating,
                    sig: new_vol,
                }
            })
            .collect();

        standings
            .into_par_iter()
            .zip(new_ratings)
            .for_each(|((player, _, _), new_rating)| {
                player.approx_posterior = new_rating;
            });
    }
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
    system: &dyn RatingSystem,
) {
    // Make sure the players exist, initializing newcomers with a default rating
    contest.standings.iter().for_each(|&(ref handle, _, _)| {
        players
            .entry(handle.clone())
            .or_insert_with(|| RefCell::new(Player::with_rating(MU_NEWBIE, SIG_NEWBIE)));
    });

    // Store guards so that the cells can be released later
    let mut guards: Vec<RefMut<Player>> = contest
        .standings
        .iter()
        .map(|&(ref handle, _, _)| players.get(handle).unwrap().borrow_mut())
        .collect();

    // Get mut references to all requested players, panic if handles are not distinct
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
        ratings.sort_by(|a, b| a.partial_cmp(b).unwrap());
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
