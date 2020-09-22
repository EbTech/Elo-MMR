use super::contest_config::Contest;
use super::compute_ratings::{RatingSystem, Rating, Player, robust_average};
use rayon::prelude::*;
use std::cell::{RefCell, RefMut};
use std::cmp::max;
use std::collections::{HashMap, VecDeque};

/// TopCoder system details: https://www.topcoder.com/community/competitive-programming/how-to-compete/ratings
/// Further analysis: https://web.archive.org/web/20120417104152/http://brucemerry.org.za:80/tc-rating/rating_submit1.pdf
pub struct TopCoderSystem {}

impl Default for TopCoderSystem {
    fn default() -> Self {
        Self {}
    }
}

impl RatingSystem for TopCoderSystem {
    fn round_update(&mut self, standings: Vec<(&mut Player, usize, usize)>) {
        use statrs::function::erf::{erfc, erfc_inv};
        use std::f64::consts::SQRT_2;

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

        let new_ratings: Vec<Rating> = standings
            .par_iter()
            .map(|(player, lo, hi)| {
                let old_rating = player.approx_posterior.mu;
                let vol_sq = player.approx_posterior.sig.powi(2);
                let win_pr = |rating: &Rating| {
                    0.5 * erfc(
                        (old_rating - rating.mu) / (2. * (vol_sq + rating.sig.powi(2))).sqrt(),
                    )
                };

                let ex_rank = standings
                    .iter()
                    .map(|&(ref foe, _, _)| (win_pr(&foe.approx_posterior)))
                    .sum::<f64>();
                let ac_rank = 0.5 * (1 + lo + hi) as f64;

                // cdf(-perf) = rank / num_coders
                //   => perf  = -inverse_cdf(rank / num_coders)
                // If perf is standard normal, we get inverse_cdf using erfc_inv:
                let ex_perf = SQRT_2 * erfc_inv(2. * ex_rank / num_coders);
                let ac_perf = SQRT_2 * erfc_inv(2. * ac_rank / num_coders);
                let perf_as = old_rating + c_factor * (ac_perf - ex_perf);

                let mut weight = 1. / (0.82 - 0.42 / player.num_contests as f64) - 1.;
                if old_rating >= 2500. {
                    weight *= 0.8;
                } else if old_rating >= 2000. {
                    weight *= 0.9;
                }
                let cap = 150. + 1500. / (player.num_contests + 1) as f64;

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