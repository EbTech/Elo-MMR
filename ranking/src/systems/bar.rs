//! The so-called Bayesian Approximation Ranking, or Algorithm 1 (BT-Full)
//! from https://jmlr.csail.mit.edu/papers/volume12/weng11a/weng11a.pdf

use super::util::{standard_logistic_cdf, Player, Rating, RatingSystem, TANH_MULTIPLIER};
use rayon::prelude::*;

#[derive(Debug)]
pub struct BAR {
    pub beta: f64,
    pub sig_drift: f64,
    pub kappa: f64,
}

impl Default for BAR {
    fn default() -> Self {
        Self {
            beta: 400. * TANH_MULTIPLIER / std::f64::consts::LN_10,
            sig_drift: 35.,
            kappa: 1e-4,
        }
    }
}

impl BAR {
    fn win_probability(&self, sig_perf: f64, player: &Rating, foe: &Rating) -> f64 {
        let c_sq = player.sig.powi(2) + foe.sig.powi(2) + 2. * sig_perf.powi(2);
        let z = (player.mu - foe.mu) / c_sq.sqrt();
        standard_logistic_cdf(z)
    }
}

impl RatingSystem for BAR {
    fn round_update(&self, contest_weight: f64, mut standings: Vec<(&mut Player, usize, usize)>) {
        let all_ratings: Vec<(Rating, usize)> = standings
            .par_iter_mut()
            .map(|(player, lo, _)| {
                player.add_noise_and_collapse(self.sig_drift);
                (player.approx_posterior, *lo)
            })
            .collect();

        let sig_perf_sq = self.beta.powi(2) / contest_weight;
        standings.into_par_iter().for_each(|(player, my_lo, _)| {
            let my_rating = &player.approx_posterior;
            let old_sig_sq = my_rating.sig.powi(2);
            let mut info = 0.;
            let mut update = 0.;
            for (rating, lo) in &all_ratings {
                let outcome = match my_lo.cmp(lo) {
                    std::cmp::Ordering::Less => 1.,
                    std::cmp::Ordering::Equal => 0.5,
                    std::cmp::Ordering::Greater => 0.,
                };
                let probability = self.win_probability(sig_perf_sq.sqrt(), my_rating, rating);

                let c_sq = old_sig_sq + rating.sig.powi(2) + 2. * sig_perf_sq;
                info += probability * (1. - probability) / c_sq;
                update += (outcome - probability) / c_sq.sqrt();
            }
            // Treat the round as one highly informative match
            info = 0.25 / (old_sig_sq + 2. * sig_perf_sq);
            update /= all_ratings.len() as f64;

            // Compute new rating deviation
            info *= old_sig_sq;
            let sig = my_rating.sig * self.kappa.max(1. - info).sqrt();

            // Compute new rating
            update *= old_sig_sq;
            let mu = my_rating.mu + update;

            player.update_rating(Rating { mu, sig }, 0.);
        });
    }
}
