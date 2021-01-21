use crate::data_processing::Contest;
use serde::{Deserialize, Serialize};
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, VecDeque};

pub const TANH_MULTIPLIER: f64 = std::f64::consts::PI / 1.7320508075688772;
pub type PlayersByName = HashMap<String, RefCell<Player>>;

#[derive(Clone, Copy, Debug)]
pub struct Rating {
    pub mu: f64,
    pub sig: f64,
}

impl Rating {
    pub fn with_noise(self, sig_noise: f64) -> Self {
        Self {
            mu: self.mu,
            sig: self.sig.hypot(sig_noise),
        }
    }

    pub fn towards_noise(self, decay: f64, limit: Self) -> Self {
        let mu_diff = self.mu - limit.mu;
        let sig_sq_diff = self.sig * self.sig - limit.sig * limit.sig;
        Self {
            mu: limit.mu + mu_diff * decay,
            sig: (self.sig * self.sig + sig_sq_diff * decay * decay).sqrt(),
        }
    }
}

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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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

    pub fn is_newcomer(&self) -> bool {
        self.event_history.len() <= 1
    }

    pub fn update_rating(&mut self, rating: Rating) {
        // Assumes that a placeholder history item has been pushed containing contest id and time
        self.approx_posterior = rating;
        let last_event = self.event_history.last_mut().unwrap();
        assert_eq!(last_event.display_rating, 0);

        // TODO: get rid of the magic numbers 2 and 80!
        //       2.0 gives a conservative estimate: use 0 to get mean estimates
        //       80 is EloR's default sig_lim
        last_event.display_rating = (rating.mu - 2.0 * (rating.sig - 80.)).round() as i32;
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
        self.approx_posterior = self.approx_posterior.with_noise(sig_noise);
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
        let new_posterior = self.approx_posterior.with_noise(sig_noise);

        let decay = (self.approx_posterior.sig / new_posterior.sig).powi(2);
        let transfer = decay.powf(transfer_speed);
        self.approx_posterior = new_posterior;

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
    0.5 * statrs::function::erf::erfc(-z / std::f64::consts::SQRT_2)
    // Less numerically stable: 0.5 + 0.5 * statrs::function::erf::erf(z / std::f64::consts::SQRT_2)
}

pub fn standard_normal_cdf_inv(prob: f64) -> f64 {
    -std::f64::consts::SQRT_2 * statrs::function::erf::erfc_inv(2. * prob)
    // Equivalently: std::f64::consts::SQRT_2 * statrs::function::erf::erf_inv(2. * prob - 1.)
}

#[allow(dead_code)]
pub fn solve_bisection((mut lo, mut hi): (f64, f64), f: impl Fn(f64) -> f64) -> f64 {
    loop {
        let flo = f(lo);
        let guess = 0.5 * (lo + hi);
        if lo >= guess || guess >= hi {
            return guess;
        }
        if f(guess) * flo > 0. {
            lo = guess;
        } else {
            hi = guess;
        }
    }
}

#[allow(dead_code)]
pub fn solve_illinois((mut lo, mut hi): (f64, f64), f: impl Fn(f64) -> f64) -> f64 {
    let (mut flo, mut fhi, mut side) = (f(lo), f(hi), 0i8);
    loop {
        let guess = (flo * hi - fhi * lo) / (flo - fhi);
        if lo >= guess || guess >= hi {
            return 0.5 * (lo + hi);
        }
        let fguess = f(guess);
        if fguess * flo > 0. {
            lo = guess;
            flo = fguess;
            if side == -1 {
                fhi *= 0.5;
            }
            side = -1;
        } else if fguess * fhi > 0. {
            hi = guess;
            fhi = fguess;
            if side == 1 {
                flo *= 0.5;
            }
            side = 1;
        } else {
            return guess;
        }
    }
}

pub fn solve_newton((mut lo, mut hi): (f64, f64), f: impl Fn(f64) -> (f64, f64)) -> f64 {
    let mut guess = 0.5 * (lo + hi);
    loop {
        let (sum, sum_prime) = f(guess);
        let extrapolate = guess - sum / sum_prime;
        if extrapolate < guess {
            hi = guess;
            guess = extrapolate.max(0.75 * lo + 0.25 * hi).min(hi);
        } else {
            lo = guess;
            guess = extrapolate.max(lo).min(0.25 * lo + 0.75 * hi);
        }
        if lo >= guess || guess >= hi {
            if sum.abs() > 1e-10 {
                eprintln!(
                    "WARNING: POSSIBLE FAILURE TO CONVERGE @ {}: s={}, s'={}",
                    guess, sum, sum_prime
                );
            }
            return guess;
        }
    }
}

// Returns the unique zero of the following strictly increasing function of x:
// offset + slope * x + sum_i weight_i * tanh((x-mu_i)/sig_i)
// We must have slope != 0 or |offset| < sum_i weight_i in order for the zero to exist.
// If offset == slope == 0, we get a robust weighted average of the mu_i's.
pub fn robust_average(
    all_ratings: impl Iterator<Item = TanhTerm> + Clone,
    offset: f64,
    slope: f64,
) -> f64 {
    let bounds = (-6000.0, 9000.0);
    let f = |x: f64| -> (f64, f64) {
        all_ratings
            .clone()
            .map(|term| {
                let tanh_z = ((x - term.mu) * term.w_arg).tanh();
                (
                    tanh_z * term.w_out,
                    (1. - tanh_z * tanh_z) * term.w_arg * term.w_out,
                )
            })
            .fold((offset + slope * x, slope), |(s, sp), (v, vp)| {
                (s + v, sp + vp)
            })
    };
    solve_newton(bounds, f)
}

pub trait RatingSystem: std::fmt::Debug {
    fn win_probability(&self, player: &Rating, foe: &Rating) -> f64;
    fn round_update(&self, standings: Vec<(&mut Player, usize, usize)>);
}

pub fn simulate_contest(
    players: &mut PlayersByName,
    contest: &Contest,
    system: &dyn RatingSystem,
    mu_newbie: f64,
    sig_newbie: f64,
) {
    if contest.standings.is_empty() || contest.standings[0].2 + 1 >= contest.standings.len() {
        eprintln!(
            "WARNING: ignoring contest {} because all players tied",
            contest.id
        );
        return;
    }

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

pub fn get_participant_ratings(
    players: &mut PlayersByName,
    contest_standings: &[(String, usize, usize)],
    min_history: usize,
) -> Vec<(Rating, usize, usize)> {
    let mut standings: Vec<(Rating, usize, usize)> = vec![];

    for &(ref handle, lo, hi) in contest_standings {
        if let Some(player) = players.get(handle).map(RefCell::borrow) {
            if player.event_history.len() >= min_history {
                standings.push((player.approx_posterior, lo, hi));
            }
        }
    }

    // Normalizing the ranks is very annoying, I probably should've just represented
    // standings as an Vec of Vec of players
    let (mut last_k, mut last_v) = (usize::MAX, usize::MAX);
    for (i, (_, lo, _)) in standings.iter_mut().enumerate() {
        if *lo != last_k {
            last_k = *lo;
            last_v = i;
        }
        *lo = last_v;
    }
    let (mut last_k, mut last_v) = (usize::MAX, usize::MAX);
    for (i, (_, _, hi)) in standings.iter_mut().enumerate().rev() {
        if *hi != last_k {
            last_k = *hi;
            last_v = i;
        }
        *hi = last_v;
    }
    standings
}
