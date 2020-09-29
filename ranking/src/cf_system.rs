//! Codeforces system details: https://codeforces.com/blog/entry/20762

use crate::compute_ratings::{robust_average, standard_logistic_cdf, Player, Rating, RatingSystem};
use rayon::prelude::*;

pub struct CodeforcesSystem {
    sig_perf: f64, // must be positive, only affects scale, since CF ignores SIG_NEWBIE
    weight: f64,   // must be positive
}

impl Default for CodeforcesSystem {
    fn default() -> Self {
        Self {
            sig_perf: 800. / std::f64::consts::LN_10,
            weight: 1.,
        }
    }
}

impl CodeforcesSystem {
    // ratings is a list of the participants, ordered from first to last place
    // returns: performance of the player in ratings[id] who tied against ratings[lo..hi]
    fn compute_performance(
        &self,
        better: &[Rating],
        worse: &[Rating],
        all: &[Rating],
        my_rating: Rating,
    ) -> f64 {
        // The conversion is 2*rank - 1/my_sig = 2*pos_offset + tied_offset = pos - neg + all
        // Note: the caller currently guarantees that every .sig equals self.sig_perf
        let pos_offset: f64 = better.iter().map(|rating| rating.sig.recip()).sum();
        let neg_offset: f64 = worse.iter().map(|rating| rating.sig.recip()).sum();
        let all_offset: f64 = all.iter().map(|rating| rating.sig.recip()).sum();

        let ac_rank = 0.5 * (pos_offset - neg_offset + all_offset + my_rating.sig.recip());
        let ex_rank = 0.5 / my_rating.sig
            + all
                .iter()
                .map(|rating| self.win_probability(rating, &my_rating) / rating.sig)
                .sum::<f64>();

        let geo_rank = (ac_rank * ex_rank).sqrt();
        let geo_offset = 2. * geo_rank - my_rating.sig.recip() - all_offset;
        let geo_rating = robust_average(all.iter().cloned().map(Into::into), geo_offset, 0.);
        geo_rating
    }
}

impl RatingSystem for CodeforcesSystem {
    fn win_probability(&self, player: &Rating, foe: &Rating) -> f64 {
        let z = (player.mu - foe.mu) / self.sig_perf;
        standard_logistic_cdf(z)
    }

    fn round_update(&self, standings: Vec<(&mut Player, usize, usize)>) {
        let all_ratings: Vec<Rating> = standings
            .par_iter()
            .map(|(player, _, _)| Rating {
                mu: player.approx_posterior.mu,
                sig: self.sig_perf,
            })
            .collect();

        standings
            .into_par_iter()
            .zip(all_ratings.par_iter())
            .for_each(|((player, lo, hi), &my_rating)| {
                let geo_perf = self.compute_performance(
                    &all_ratings[..lo],
                    &all_ratings[hi + 1..],
                    &all_ratings,
                    my_rating,
                );
                let mu = (my_rating.mu + self.weight * geo_perf) / (1. + self.weight);
                let sig = player.approx_posterior.sig;
                player.update_rating(Rating { mu, sig });
            });
    }
}
