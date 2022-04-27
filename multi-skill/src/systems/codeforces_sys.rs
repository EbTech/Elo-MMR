//! Codeforces system details: https://codeforces.com/blog/entry/20762

use super::{robust_average, Player, Rating, RatingSystem};
use crate::numerical::{standard_logistic_cdf, TANH_MULTIPLIER};
use rayon::prelude::*;

#[derive(Debug)]
pub struct CodeforcesSys {
    pub beta: f64,   // must be positive, only affects scale, since CF ignores SIG_NEWBIE
    pub weight: f64, // must be positive
}

impl Default for CodeforcesSys {
    fn default() -> Self {
        Self {
            beta: 400. * TANH_MULTIPLIER / std::f64::consts::LN_10,
            weight: 1.,
        }
    }
}

impl CodeforcesSys {
    // ratings is a list of the participants, ordered from first to last place
    // returns: performance of the player in ratings[id] who tied against ratings[lo..hi]
    fn compute_performance(
        &self,
        sig_perf: f64,
        better: &[Rating],
        worse: &[Rating],
        all: &[Rating],
        my_rating: Rating,
    ) -> f64 {
        // The conversion is 2*rank - 1/my_sig = 2*pos_offset + tied_offset = pos - neg + all
        // Note: the caller currently guarantees that every .sig equals sig_perf
        let pos_offset: f64 = better.iter().map(|rating| rating.sig.recip()).sum();
        let neg_offset: f64 = worse.iter().map(|rating| rating.sig.recip()).sum();
        let all_offset: f64 = all.iter().map(|rating| rating.sig.recip()).sum();

        let ac_rank = 0.5 * (pos_offset - neg_offset + all_offset + my_rating.sig.recip());
        let ex_rank = 0.5 / my_rating.sig
            + all
                .iter()
                .map(|rating| self.win_probability(sig_perf, rating, &my_rating) / rating.sig)
                .sum::<f64>();

        let geo_rank = (ac_rank * ex_rank).sqrt();
        let geo_offset = 2. * geo_rank - my_rating.sig.recip() - all_offset;
        let geo_rating = robust_average(
            all.iter().cloned().map(Into::into),
            TANH_MULTIPLIER * geo_offset,
            0.,
        );
        geo_rating
    }

    fn win_probability(&self, sig_perf: f64, player: &Rating, foe: &Rating) -> f64 {
        let z = (player.mu - foe.mu) / sig_perf;
        standard_logistic_cdf(z)
    }
}

impl RatingSystem for CodeforcesSys {
    fn round_update(&self, contest_weight: f64, standings: Vec<(&mut Player, usize, usize)>) {
        let sig_perf = self.beta / contest_weight.sqrt();
        let all_ratings: Vec<Rating> = standings
            .par_iter()
            .map(|(player, _, _)| Rating {
                mu: player.approx_posterior.mu,
                sig: sig_perf,
            })
            .collect();

        standings
            .into_par_iter()
            .zip(all_ratings.par_iter())
            .for_each(|((player, lo, hi), &my_rating)| {
                let geo_perf = self.compute_performance(
                    sig_perf,
                    &all_ratings[..lo],
                    &all_ratings[hi + 1..],
                    &all_ratings,
                    my_rating,
                );
                let wt = contest_weight * self.weight;
                let mu = (my_rating.mu + wt * geo_perf) / (1. + wt);
                let sig = player.approx_posterior.sig;
                player.update_rating(Rating { mu, sig }, geo_perf);
            });
    }
}
