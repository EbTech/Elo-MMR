extern crate overload;

use overload::overload;
use std::fmt;
use std::ops;

use crate::contest_config::Contest;
use std::cell::{Ref, RefCell, RefMut};
use std::cmp::min;
use std::collections::{HashMap, VecDeque};

pub const TANH_MULTIPLIER: f64 = std::f64::consts::PI / 1.7320508075688772;

#[derive(Clone, Copy, Debug)]
pub struct Rating {
    pub mu: f64,
    pub sig: f64,
}

// A data structure for storing the various performance metrics we want to
// analyze
pub struct PerformanceReport {
    pub topk: f64,
    pub percentile: f64,
    pub nrounds: f64,
}

impl Default for PerformanceReport {
    fn default() -> Self {
        Self {
            topk: 0.,
            percentile: 0.,
            nrounds: 0.,
        }
    }
}

impl fmt::Display for PerformanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.topk, self.percentile)
    }
}

overload!((a: ?PerformanceReport) + (b: ?PerformanceReport) -> PerformanceReport {
    PerformanceReport {
        topk: (a.topk * a.nrounds + b.topk * b.nrounds) / (a.nrounds + b.nrounds),
        percentile: (a.percentile * a.nrounds + b.percentile * b.nrounds) / (a.nrounds + b.nrounds),
        nrounds: a.nrounds + b.nrounds,
    }
});

overload!((a: &mut PerformanceReport) += (b: ?PerformanceReport) {
    a.topk = (a.topk * a.nrounds + b.topk * b.nrounds) / (a.nrounds + b.nrounds);
    a.percentile = (a.percentile * a.nrounds + b.percentile * b.nrounds) / (a.nrounds + b.nrounds);
    a.nrounds += b.nrounds;
});

#[derive(Clone, Copy, Debug)]
pub struct TanhTerm {
    pub mu: f64,
    pub w_arg: f64,
    pub w_out: f64,
}

impl From<Rating> for TanhTerm {
    fn from(rating: Rating) -> Self {
        let w = TANH_MULTIPLIER / rating.sig;
        Self {
            mu: rating.mu,
            w_arg: w * 0.5,
            w_out: w,
        }
    }
}

impl TanhTerm {
    pub fn get_weight(&self) -> f64 {
        self.w_arg * self.w_out * 2. / TANH_MULTIPLIER.powi(2)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PlayerEvent {
    pub contest_id: usize,
    pub contest_time: u64,
    pub display_rating: i32,
}

pub struct Player {
    // TODO: mark all fields private, with API based on appropriately-named read-only methods
    normal_factor: Rating,
    pub logistic_factors: VecDeque<TanhTerm>,
    pub event_history: Vec<PlayerEvent>,
    pub approx_posterior: Rating,
}

impl Player {
    pub fn with_rating(mu: f64, sig: f64) -> Self {
        Player {
            normal_factor: Rating { mu, sig },
            logistic_factors: VecDeque::new(),
            event_history: vec![],
            approx_posterior: Rating { mu, sig },
        }
    }

    pub fn update_rating(&mut self, rating: Rating) {
        // Assumes that a placeholder history item has been pushed containing contest id and time
        self.approx_posterior = rating;
        let last_event = self.event_history.last_mut().unwrap();
        assert_eq!(last_event.display_rating, 0);

        // TODO: get rid of the magic numbers 2 and 80!
        //       2 gives a conservative estimate: use 0 to get mean estimates
        //       80 is EloR's default sig_lim
        last_event.display_rating = (rating.mu - 2. * (rating.sig - 80.)).round() as i32;
    }

    pub fn update_rating_with_new_performance(&mut self, performance: Rating, max_history: usize) {
        if self.logistic_factors.len() >= max_history {
            // wl can be chosen so as to preserve total weight or rating; we choose the former.
            // Either way, the deleted element should be small enough not to matter.
            let logistic = self.logistic_factors.pop_front().unwrap();
            let wn = self.normal_factor.sig.powi(-2);
            let wl = logistic.get_weight();
            self.normal_factor.mu = (wn * self.normal_factor.mu + wl * logistic.mu) / (wn + wl);
            self.normal_factor.sig = (wn + wl).recip().sqrt();
        }
        self.logistic_factors.push_back(performance.into());

        let weight = self.normal_factor.sig.powi(-2);
        let mu = robust_average(
            self.logistic_factors.iter().cloned(),
            -self.normal_factor.mu * weight,
            weight,
        );
        let sig = (self.approx_posterior.sig.powi(-2) + performance.sig.powi(-2))
            .recip()
            .sqrt();
        self.update_rating(Rating { mu, sig });
    }

    // Method #1: the Gaussian/Brownian approximation, in which rating is a Markov state
    // Equivalent to method #5 with transfer_speed == f64::INFINITY
    pub fn add_noise_and_collapse(&mut self, sig_noise: f64) {
        self.approx_posterior.sig = self.approx_posterior.sig.hypot(sig_noise);
        self.normal_factor = self.approx_posterior;
        self.logistic_factors.clear();
    }

    // Method #2: decrease weights without changing logistic sigmas
    // Equivalent to method #5 with transfer_speed == 0
    #[allow(dead_code)]
    pub fn add_noise_in_front(&mut self, sig_noise: f64) {
        let decay = 1.0f64.hypot(sig_noise / self.approx_posterior.sig);
        self.approx_posterior.sig *= decay;

        self.normal_factor.sig *= decay;
        for rating in &mut self.logistic_factors {
            rating.w_out /= decay * decay;
        }
    }

    // #5: a general method with the nicest properties, parametrized by transfer_speed >= 0
    // Reduces to method #1 when transfer_speed == f64::INFINITY
    // Reduces to method #2 when transfer_speed == 0
    pub fn add_noise_best(&mut self, sig_noise: f64, transfer_speed: f64) {
        let new_sig = self.approx_posterior.sig.hypot(sig_noise);

        let decay = (self.approx_posterior.sig / new_sig).powi(2);
        let transfer = decay.powf(transfer_speed);
        self.approx_posterior.sig = new_sig;

        let wt_norm_old = self.normal_factor.sig.powi(-2);
        let wt_from_norm_old = transfer * wt_norm_old;
        let wt_from_transfers = (1. - transfer)
            * (wt_norm_old
                + self
                    .logistic_factors
                    .iter()
                    .map(TanhTerm::get_weight)
                    .sum::<f64>());
        let wt_total = wt_from_norm_old + wt_from_transfers;

        self.normal_factor.mu = (wt_from_norm_old * self.normal_factor.mu
            + wt_from_transfers * self.approx_posterior.mu)
            / wt_total;
        self.normal_factor.sig = (decay * wt_total).recip().sqrt();
        for r in &mut self.logistic_factors {
            r.w_out *= transfer * decay;
        }
    }
}

#[allow(dead_code)]
pub fn standard_logistic_pdf(z: f64) -> f64 {
    0.25 * TANH_MULTIPLIER * (0.5 * TANH_MULTIPLIER * z).cosh().powi(-2)
}

pub fn standard_logistic_cdf(z: f64) -> f64 {
    0.5 + 0.5 * (0.5 * TANH_MULTIPLIER * z).tanh()
}

#[allow(dead_code)]
pub fn standard_logistic_cdf_inv(prob: f64) -> f64 {
    (2. * prob - 1.).atanh() * 2. / TANH_MULTIPLIER
}

pub fn standard_normal_pdf(z: f64) -> f64 {
    const NORMALIZE: f64 = 0.5 * std::f64::consts::FRAC_2_SQRT_PI / std::f64::consts::SQRT_2;
    NORMALIZE * (-0.5 * z * z).exp()
}

pub fn standard_normal_cdf(z: f64) -> f64 {
    // Equivalently, 0.5 * erfc(-z / SQRT_2)
    0.5 + 0.5 * statrs::function::erf::erf(z / std::f64::consts::SQRT_2)
}

pub fn standard_normal_cdf_inv(prob: f64) -> f64 {
    // Equivalently, -SQRT_2 * erfc_inv(2. * prob)
    std::f64::consts::SQRT_2 * statrs::function::erf::erf_inv(2. * prob - 1.)
}

// Returns the unique zero of the following strictly increasing function of x:
// offset + slope * x + sum_i weight_i * tanh((x-mu_i)/sig_i)
// We must have slope != 0 or |offset| < sum_i weight_i in order for the zero to exist.
// If offset == slope == 0, we get a robust weighted average of the mu_i's. Uses hybrid of
// binary search (to converge in the worst-case) and Newton's method (for speed in the typical case).
pub fn robust_average(
    all_ratings: impl Iterator<Item = TanhTerm> + Clone,
    offset: f64,
    slope: f64,
) -> f64 {
    let (mut lo, mut hi) = (-1000.0, 4500.0);
    let mut guess = 0.5 * (lo + hi);
    loop {
        let mut sum = offset + slope * guess;
        let mut sum_prime = slope;
        for term in all_ratings.clone() {
            let tanh_z = ((guess - term.mu) * term.w_arg).tanh();
            sum += tanh_z * term.w_out;
            sum_prime += (1. - tanh_z * tanh_z) * term.w_arg * term.w_out;
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
    fn round_update(&self, standings: Vec<(&mut Player, usize, usize)>);
    fn compute_metrics(
        &self,
        standings: Vec<(&Player, usize, usize)>,
        k: i32,
    ) -> PerformanceReport {
        let mut ranks: Vec<(f64, usize)> = Vec::<(f64, usize)>::new();
        for i in 0..standings.len() {
            let pa = standings[i].0.approx_posterior;
            ranks.push((-pa.mu, i));
        }
        ranks.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Compute topk (frac. of inverted pairs) metric
        let kreal = min(k as usize, standings.len());
        let mut pairs_correct = 0.;
        let tot_pairs = (kreal * (kreal - 1)) as f64 / 2.;
        for i in 0..standings.len() {
            if ranks[i].1 >= k as usize {
                continue;
            }
            for j in i + 1..standings.len() {
                if ranks[j].1 >= k as usize {
                    continue;
                }
                //println!("{} {} {} {} {}", i, j, ranks[i].1, ranks[j].1, ranks[i].1 < ranks[j].1);
                if ranks[i].1 < ranks[j].1 {
                    pairs_correct += 1.;
                }
            }
        }

        // Compute avg percentile distance metric
        let mut avg_percent = 0.;
        for i in 0..ranks.len() {
            avg_percent += (i as f64 - ranks[i].1 as f64).abs() / ranks.len() as f64;
        }
        avg_percent /= ranks.len() as f64;

        PerformanceReport {
            topk: pairs_correct / tot_pairs,
            percentile: avg_percent,
            nrounds: 1.,
        }
    }
}

pub fn simulate_contest(
    players: &mut HashMap<String, RefCell<Player>>,
    contest: &Contest,
    system: &mut dyn RatingSystem,
    mu_newbie: f64,
    sig_newbie: f64,
) {
    // If a player is competing for the first time, initialize with a default rating
    contest.standings.iter().for_each(|&(ref handle, _, _)| {
        players
            .entry(handle.clone())
            .or_insert_with(|| RefCell::new(Player::with_rating(mu_newbie, sig_newbie)));
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
            player.event_history.push(PlayerEvent {
                contest_id: contest.id,
                contest_time: contest.time_seconds,
                display_rating: 0,
            });
            std::ops::DerefMut::deref_mut(player)
        })
        .zip(contest.standings.iter())
        .map(|(player, &(_, lo, hi))| (player, lo, hi))
        .collect();

    system.round_update(standings);
}

pub fn predict_performance(
    players: &mut HashMap<String, RefCell<Player>>,
    contest: &Contest,
    system: &dyn RatingSystem,
    mu_newbie: f64,
    sig_newbie: f64,
    topk: i32,
) -> PerformanceReport {
    // If a player is competing for the first time, initialize with a default rating
    contest.standings.iter().for_each(|&(ref handle, _, _)| {
        players
            .entry(handle.clone())
            .or_insert_with(|| RefCell::new(Player::with_rating(mu_newbie, sig_newbie)));
    });

    let guards: Vec<Ref<Player>> = contest
        .standings
        .iter()
        .map(|&(ref handle, _, _)| players.get(handle).expect("Duplicate handles").borrow())
        .collect();

    // Update player metadata and get &mut references to all requested players
    let standings: Vec<(&Player, usize, usize)> = guards
        .iter()
        .map(|player| std::ops::Deref::deref(player))
        .zip(contest.standings.clone())
        .map(|(player, (_, lo, hi))| (player, lo, hi))
        .collect();

    system.compute_metrics(standings, topk)
}
