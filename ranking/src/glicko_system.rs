//! Glicko system details: https://en.wikipedia.org/wiki/Glicko_rating_system

use crate::compute_ratings::{
    standard_logistic_cdf, Player, Rating, RatingSystem, TANH_MULTIPLIER,
};
use rayon::prelude::*;

#[derive(Debug)]
pub struct GlickoSystem {
    pub sig_perf: f64,
    pub sig_drift: f64,
}

impl Default for GlickoSystem {
    fn default() -> Self {
        Self {
            sig_perf: 400. * TANH_MULTIPLIER / std::f64::consts::LN_10,
            sig_drift: 35.,
        }
    }
}

impl RatingSystem for GlickoSystem {
    fn win_probability(&self, player: &Rating, foe: &Rating) -> f64 {
        let z = (player.mu - foe.mu) / foe.sig.hypot(self.sig_perf);
        standard_logistic_cdf(z)
    }

    fn round_update(&self, mut standings: Vec<(&mut Player, usize, usize)>) {
        let all_ratings: Vec<(Rating, usize, f64)> = standings
            .par_iter_mut()
            .map(|(player, lo, _)| {
                player.add_noise_and_collapse(self.sig_drift);
                let g = 1f64
                    .hypot(player.approx_posterior.sig / self.sig_perf)
                    .recip();
                (player.approx_posterior, *lo, g)
            })
            .collect();

        let gli_q = TANH_MULTIPLIER / self.sig_perf;
        standings.into_par_iter().for_each(|(player, my_lo, _)| {
            let my_rating = &player.approx_posterior;
            let mut info = 0.;
            let mut update = 0.;
            for (rating, lo, g) in &all_ratings {
                let outcome = match my_lo.cmp(lo) {
                    std::cmp::Ordering::Less => 1.,
                    std::cmp::Ordering::Equal => 0.5,
                    std::cmp::Ordering::Greater => 0.,
                };
                let probability = self.win_probability(my_rating, rating);
                // Equivalently, let probability =
                //  (1f64 + (gli_q * g * (rating.mu - my_rating.mu)).exp()).recip();

                info += g * g * probability * (1. - probability);
                update += g * (outcome - probability);
            }
            // Treat the round as one highly informative match
            info = 0.25;
            update /= all_ratings.len() as f64;

            // Compute new rating deviation
            info *= gli_q * gli_q;
            let sig = (my_rating.sig.powi(-2) + info).recip().sqrt();

            // Compute new rating
            update *= gli_q * sig * sig;
            let mu = my_rating.mu + update;

            player.update_rating(Rating { mu, sig });
        });
    }
}
