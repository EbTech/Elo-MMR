//! Topcoder system details: https://www.topcoder.com/community/competitive-programming/how-to-compete/ratings
//! Further analysis: https://web.archive.org/web/20120417104152/http://brucemerry.org.za:80/tc-rating/rating_submit1.pdf

use super::util::{standard_normal_cdf, standard_normal_cdf_inv, Player, Rating, RatingSystem};
use rayon::prelude::*;

#[derive(Debug)]
pub struct TopcoderSys {
    pub weight_multiplier: f64, // must be positive
}

impl Default for TopcoderSys {
    fn default() -> Self {
        Self {
            weight_multiplier: 1.,
        }
    }
}

impl RatingSystem for TopcoderSys {
    fn win_probability(&self, player: &Rating, foe: &Rating) -> f64 {
        let z = (player.mu - foe.mu) / player.sig.hypot(foe.sig);
        standard_normal_cdf(z)
    }

    fn round_update(&self, standings: Vec<(&mut Player, usize, usize)>) {
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

        let limit_weight = 1. / 0.82 - 1.;
        let cap_multiplier = self.weight_multiplier * (1. + limit_weight)
            / (1. + limit_weight * self.weight_multiplier);

        let new_ratings: Vec<Rating> = standings
            .par_iter()
            .map(|(player, lo, hi)| {
                let old_rating = player.approx_posterior.mu;
                let vol_sq = player.approx_posterior.sig.powi(2);

                let ex_rank = standings
                    .iter()
                    .map(|&(ref foe, _, _)| {
                        self.win_probability(&foe.approx_posterior, &player.approx_posterior)
                    })
                    .sum::<f64>();
                let ac_rank = 0.5 * (1 + lo + hi) as f64;

                // cdf(-perf) = rank / num_coders
                //   => perf  = -inverse_cdf(rank / num_coders)
                let ex_perf = -standard_normal_cdf_inv(ex_rank / num_coders);
                let ac_perf = -standard_normal_cdf_inv(ac_rank / num_coders);
                let perf_as = old_rating + c_factor * (ac_perf - ex_perf);

                let num_contests = player.event_history.len() as f64;
                let mut weight = 1. / (0.82 - 0.42 / num_contests) - 1.;
                weight *= self.weight_multiplier;
                if old_rating >= 2500. {
                    weight *= 0.8;
                } else if old_rating >= 2000. {
                    weight *= 0.9;
                }

                let mut cap = 150. + 1500. / (num_contests + 1.);
                cap *= cap_multiplier;

                let try_rating = (old_rating + weight * perf_as) / (1. + weight);
                let new_rating = try_rating.max(old_rating - cap).min(old_rating + cap);
                let new_vol =
                    ((try_rating - old_rating).powi(2) / weight + vol_sq / (1. + weight)).sqrt();

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
                player.update_rating(new_rating);
            });
    }
}
