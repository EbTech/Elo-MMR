use super::compute_ratings::{Player, Rating, RatingSystem};
use rayon::prelude::*;
use statrs::function::erf::{erfc, erfc_inv};
use std::f64::consts::SQRT_2;

/// TopCoder system details: https://www.topcoder.com/community/competitive-programming/how-to-compete/ratings
/// Further analysis: https://web.archive.org/web/20120417104152/http://brucemerry.org.za:80/tc-rating/rating_submit1.pdf
pub struct TopCoderSystem {
    weight_multiplier: f64,
}

impl Default for TopCoderSystem {
    fn default() -> Self {
        Self {
            weight_multiplier: 1.,
        }
    }
}

impl RatingSystem for TopCoderSystem {
    fn win_probability(&self, player: &Rating, foe: &Rating) -> f64 {
        0.5 * erfc((foe.mu - player.mu) / foe.sig.hypot(player.sig) / SQRT_2)
    }

    fn round_update(&mut self, standings: Vec<(&mut Player, usize, usize)>) {
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
                // If perf is standard normal, we get inverse_cdf using erfc_inv:
                let ex_perf = SQRT_2 * erfc_inv(2. * ex_rank / num_coders);
                let ac_perf = SQRT_2 * erfc_inv(2. * ac_rank / num_coders);
                let perf_as = old_rating + c_factor * (ac_perf - ex_perf);

                let mut weight = 1. / (0.82 - 0.42 / player.num_contests as f64) - 1.;
                weight *= self.weight_multiplier;
                if old_rating >= 2500. {
                    weight *= 0.8;
                } else if old_rating >= 2000. {
                    weight *= 0.9;
                }

                let mut cap = 150. + 1500. / (player.num_contests + 1) as f64;
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
                player.approx_posterior = new_rating;
            });
    }
}
