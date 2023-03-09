//! Endure-Elo system details: https://www-users.york.ac.uk/~bp787/Generalizing_Elo_arxiv.pdf

use super::{Player, Rating, RatingSystem};
use crate::data_processing::ContestRatingParams;
use crate::numerical::{standard_logistic_cdf, TANH_MULTIPLIER};
use rayon::prelude::*;

#[derive(Debug)]
pub struct EndureElo {
    pub beta: f64,
    pub sig_drift: f64,
}

impl Default for EndureElo {
    fn default() -> Self {
        Self {
            beta: 400. * TANH_MULTIPLIER / std::f64::consts::LN_10,
            sig_drift: 35.,
        }
    }
}

impl EndureElo {
    fn win_probability(&self, sig_perf: f64, player: &Rating, foe: &Rating) -> f64 {
        let z = (player.mu - foe.mu) / foe.sig.hypot(sig_perf);
        standard_logistic_cdf(z)
    }
}

impl RatingSystem for EndureElo {
    fn round_update(
        &self,
        params: ContestRatingParams,
        mut standings: Vec<(&mut Player, usize, usize)>,
    ) {
        unimplemented!("The EndureElo system has only skeleton code!");

        standings.par_iter_mut().for_each(|(player, lo, _)| {
            player.add_noise_and_collapse(self.sig_drift);
        });

        standings.into_par_iter().for_each(|(player, my_lo, _)| {
            let my_rating = &player.approx_posterior;
            let probability = 0.5;
            let info = probability * (1.0 - probability);
            let sig = (my_rating.sig.powi(-2) + info).recip().sqrt();

            // Compute new rating
            let mu = my_rating.mu; // + update;

            player.update_rating(Rating { mu, sig }, 0.);
        });
    }
}
