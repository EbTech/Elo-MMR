//! This version has fewer features and optimizations than elo_mmr.rs, more
//! closely matching the pseudocode in https://arxiv.org/abs/2101.00400
use super::{Player, Rating, RatingSystem, TanhTerm};
use crate::data_processing::ContestRatingParams;
use crate::numerical::solve_newton;
use rayon::prelude::*;

fn eval_less(term: &TanhTerm, x: f64) -> (f64, f64) {
    let (val, val_prime) = term.base_values(x);
    (val - term.w_out, val_prime)
}

fn eval_grea(term: &TanhTerm, x: f64) -> (f64, f64) {
    let (val, val_prime) = term.base_values(x);
    (val + term.w_out, val_prime)
}

fn eval_equal(term: &TanhTerm, x: f64, mul: f64) -> (f64, f64) {
    let (val, val_prime) = term.base_values(x);
    (mul * val, mul * val_prime)
}

#[derive(Debug)]
pub struct SimpleEloMMR {
    // the weight of each new contest
    pub weight_limit: f64,
    // weight multipliers (less than one) to apply on first few contests
    pub noob_delay: Vec<f64>,
    // each contest participation adds an amount of drift such that, in the absence of
    // much time passing, the limiting skill uncertainty's square approaches this value
    pub sig_limit: f64,
    // additional variance per second, from a drift that's continuous in time
    pub drift_per_sec: f64,
    // whether to count ties as half a win plus half a loss
    pub split_ties: bool,
    // maximum number of recent contests to store, must be at least 1
    pub history_len: usize,
    // maximum number of opponents and recent events to use, as a compute-saving approximation
    pub transfer_speed: f64,
}

impl Default for SimpleEloMMR {
    fn default() -> Self {
        Self {
            weight_limit: 0.2,
            noob_delay: vec![0.6, 0.8], // TODO: make the default empty once it's configurable
            sig_limit: 80.,
            drift_per_sec: 0.,
            split_ties: false,
            history_len: usize::MAX,
            transfer_speed: 1.,
        }
    }
}

impl SimpleEloMMR {
    fn sig_perf_and_drift(&self, mut contest_weight: f64, n: usize) -> (f64, f64) {
        contest_weight *= self.weight_limit;
        contest_weight *= self.noob_delay.get(n).unwrap_or(&1.);
        let sig_perf = (1. + 1. / contest_weight).sqrt() * self.sig_limit;
        let sig_drift_sq = contest_weight * self.sig_limit * self.sig_limit;
        (sig_perf, sig_drift_sq)
    }
}

impl RatingSystem for SimpleEloMMR {
    fn round_update(
        &self,
        params: ContestRatingParams,
        mut standings: Vec<(&mut Player, usize, usize)>,
    ) {
        // Update ratings due to waiting period between contests,
        // then use it to create Gaussian terms for the Q-function.
        // The rank must also be stored in order to determine if it's a win,
        // loss, or tie term.
        let tanh_terms: Vec<TanhTerm> = standings
            .par_iter_mut()
            .map(|(player, _, _)| {
                let (sig_perf, discrete_drift) =
                    self.sig_perf_and_drift(params.weight, player.times_played_excl());
                let continuous_drift = self.drift_per_sec * player.update_time as f64;
                let sig_drift = (discrete_drift + continuous_drift).sqrt();
                player.add_noise_best(sig_drift, self.transfer_speed);
                player.approx_posterior.with_noise(sig_perf).into()
            })
            .collect();
        let mul = if self.split_ties { 1. } else { 2. };

        // The computational bottleneck: update ratings based on contest performance
        standings.into_par_iter().for_each(|(player, lo, hi)| {
            let bounds = (-6000.0, 9000.0);
            let f = |x| {
                let itr1 = tanh_terms[0..lo].iter().map(|term| eval_less(term, x));
                let itr2 = tanh_terms[lo..=hi]
                    .iter()
                    .map(|term| eval_equal(term, x, mul));
                let itr3 = tanh_terms[hi + 1..].iter().map(|term| eval_grea(term, x));
                itr1.chain(itr2)
                    .chain(itr3)
                    .fold((0., 0.), |(s, sp), (v, vp)| (s + v, sp + vp))
            };
            let mu_perf = solve_newton(bounds, f).min(params.perf_ceiling);
            let (sig_perf, _) = self.sig_perf_and_drift(params.weight, player.times_played_excl());
            player.update_rating_with_logistic(
                Rating {
                    mu: mu_perf,
                    sig: sig_perf,
                },
                self.history_len,
            );
        });
    }
}
