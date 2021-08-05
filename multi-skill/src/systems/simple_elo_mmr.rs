//! This version has fewer features and optimizations than elo_mmr.rs, more
//! closely matching the pseudocode in https://arxiv.org/abs/2101.00400
use super::util::{solve_newton, Player, Rating, RatingSystem, TanhTerm};
use rayon::prelude::*;

fn eval_less(term: &TanhTerm, x: f64) -> (f64, f64) {
    let (val, val_prime) = term.base_values(x);
    (val - term.w_out, val_prime)
}

fn eval_grea(term: &TanhTerm, x: f64) -> (f64, f64) {
    let (val, val_prime) = term.base_values(x);
    (val + term.w_out, val_prime)
}

fn eval_equal(term: &TanhTerm, x: f64) -> (f64, f64) {
    let (val, val_prime) = term.base_values(x);
    (2. * val, 2. * val_prime)
}

#[derive(Debug)]
pub struct SimpleEloMMR {
    // beta must exceed sig_limit
    // squared variation in individual performances, when the contest_weight is 1
    pub beta: f64,
    // each contest participation adds an amount of drift such that, in the absence of
    // much time passing, the limiting skill uncertainty's square approaches this value
    pub sig_limit: f64,
    // additional variance per second, from a drift that's continuous in time
    pub drift_per_sec: f64,
    // maximum number of opponents and recent events to use, as a compute-saving approximation
    pub transfer_speed: f64,
}

impl Default for SimpleEloMMR {
    fn default() -> Self {
        Self {
            beta: 200.,
            sig_limit: 80.,
            drift_per_sec: 0.,
            transfer_speed: 1.,
        }
    }
}

impl SimpleEloMMR {
    fn sig_perf_and_drift(&self, contest_weight: f64) -> (f64, f64) {
        let excess_beta_sq =
            (self.beta * self.beta - self.sig_limit * self.sig_limit) / contest_weight;
        let sig_perf = (self.sig_limit * self.sig_limit + excess_beta_sq).sqrt();
        let discrete_drift = self.sig_limit.powi(4) / excess_beta_sq;
        (sig_perf, discrete_drift)
    }
}

impl RatingSystem for SimpleEloMMR {
    fn round_update(&self, contest_weight: f64, mut standings: Vec<(&mut Player, usize, usize)>) {
        let (sig_perf, discrete_drift) = self.sig_perf_and_drift(contest_weight);

        // Update ratings due to waiting period between contests,
        // then use it to create Gaussian terms for the Q-function.
        // The rank must also be stored in order to determine if it's a win,
        // loss, or tie term.
        let tanh_terms: Vec<TanhTerm> = standings
            .par_iter_mut()
            .map(|(player, _, _)| {
                let continuous_drift = self.drift_per_sec * player.update_time as f64;
                let sig_drift = (discrete_drift + continuous_drift).sqrt();
                player.add_noise_best(sig_drift, self.transfer_speed);
                player.approx_posterior.with_noise(sig_perf).into()
            })
            .collect();

        // The computational bottleneck: update ratings based on contest performance
        standings.into_par_iter().for_each(|(player, lo, hi)| {
            let bounds = (-6000.0, 9000.0);
            let f = |x| {
                let itr1 = tanh_terms[0..lo].iter().map(|term| eval_less(term, x));
                let itr2 = tanh_terms[lo..=hi].iter().map(|term| eval_equal(term, x));
                let itr3 = tanh_terms[hi + 1..].iter().map(|term| eval_grea(term, x));
                itr1.chain(itr2)
                    .chain(itr3)
                    .fold((0., 0.), |(s, sp), (v, vp)| (s + v, sp + vp))
            };
            let mu_perf = solve_newton(bounds, f);
            player.update_rating_with_logistic(
                Rating {
                    mu: mu_perf,
                    sig: sig_perf,
                },
                usize::MAX,
            );
        });
    }
}
