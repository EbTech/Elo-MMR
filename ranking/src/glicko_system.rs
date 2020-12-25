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
        let z = (player.mu - foe.mu) / self.sig_perf;
        standard_logistic_cdf(z)
    }

    fn round_update(&self, mut standings: Vec<(&mut Player, usize, usize)>) {
        let all_ratings: Vec<(Rating, usize)> = standings
            .par_iter_mut()
            .map(|(player, lo, _)| {
                player.add_noise_and_collapse(self.sig_drift);
                (
                    Rating {
                        mu: player.approx_posterior.mu,
                        sig: 1f64
                            .hypot(player.approx_posterior.sig / self.sig_perf)
                            .recip(),
                    },
                    *lo,
                )
            })
            .collect();

        let glicko_q = self.sig_perf.recip() * TANH_MULTIPLIER;
        standings.into_par_iter().for_each(|(player, my_lo, _)| {
            let my_rating = player.approx_posterior;
            let mut update = 0.;
            let mut info = 0.;
            for (rating, lo) in &all_ratings {
                let g = rating.sig;
                let outcome = if my_lo < *lo {
                    1.
                } else if my_lo > *lo {
                    0.
                } else {
                    0.5
                };
                let probability =
                    (1f64 + (glicko_q * g * (rating.mu - my_rating.mu).exp())).recip();

                update += g * (outcome - probability);
                info += g * g * probability * (1. - probability);
            }
            update /= all_ratings.len() as f64;
            info /= all_ratings.len() as f64;

            let d_invsq = glicko_q * glicko_q * info;
            let sig = (my_rating.sig.powi(-2) + d_invsq).recip().sqrt();
            let weight = glicko_q * sig * sig;
            let mu = my_rating.mu + weight * update;
            player.update_rating(Rating { mu, sig });
        });
    }
}
