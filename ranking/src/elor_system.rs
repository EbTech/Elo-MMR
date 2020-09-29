//! Elo-R system details: https://github.com/EbTech/EloR/blob/master/paper/EloR.pdf

use crate::compute_ratings::{
    robust_average, standard_logistic_cdf, standard_normal_cdf, standard_normal_pdf, Player,
    Rating, RatingSystem,
};
use rayon::prelude::*;

pub enum EloRVariant {
    Gaussian,
    Logistic(f64),
}

pub struct EloRSystem {
    sig_perf: f64,        // variation in individual performances
    sig_drift: f64,       // skill drift between successive performances
    variant: EloRVariant, // whether to use logistic or Gaussian distributions
}

impl Default for EloRSystem {
    fn default() -> Self {
        Self::from_limit(250., 100., EloRVariant::Logistic(1.))
    }
}

impl EloRSystem {
    // sig_perf must exceed sig_limit, the limiting uncertainty for a player with long history
    // the ratio (sig_limit / sig_perf) effectively determines the rating update weight
    pub fn from_limit(sig_perf: f64, sig_limit: f64, variant: EloRVariant) -> Self {
        assert!(sig_limit > 0.);
        assert!(sig_perf > sig_limit);
        let sig_drift =
            ((sig_limit.powi(-2) - sig_perf.powi(-2)).recip() - sig_limit.powi(2)).sqrt();
        Self {
            sig_perf,
            sig_drift,
            variant,
        }
    }

    // Given the participants which beat us, tied with us, and lost against us,
    // returns our Gaussian-weighted performance score for this round
    fn compute_performance_gaussian(
        better: impl Iterator<Item = Rating> + Clone,
        tied: impl Iterator<Item = Rating> + Clone,
        worse: impl Iterator<Item = Rating> + Clone,
    ) -> f64 {
        // This is a slow binary search, without Newton steps
        let (mut lo, mut hi) = (-1000.0, 4500.0);
        while hi - lo > 1e-9 {
            let guess = 0.5 * (lo + hi);
            let mut sum = 0.;
            for rating in better.clone() {
                let z = (guess - rating.mu) / rating.sig;
                let pdf = standard_normal_pdf(z) / rating.sig;
                let cdf = standard_normal_cdf(z);
                sum += pdf / (cdf - 1.);
            }
            for rating in tied.clone() {
                let z = (guess - rating.mu) / rating.sig;
                let pdf = standard_normal_pdf(z) / rating.sig;
                let pdf_prime = -z * pdf / rating.sig;
                sum += pdf_prime / pdf;
            }
            for rating in worse.clone() {
                let z = (guess - rating.mu) / rating.sig;
                let pdf = standard_normal_pdf(z) / rating.sig;
                let cdf = standard_normal_cdf(z);
                sum += pdf / cdf;
            }
            if sum < 0.0 {
                hi = guess;
            } else {
                lo = guess;
            }
        }
        0.5 * (lo + hi)
    }

    // Given the participants which beat us, tied with us, and lost against us,
    // returns our logistic-weighted performance score for this round
    fn compute_performance_logistic(
        better: impl Iterator<Item = Rating> + Clone,
        tied: impl Iterator<Item = Rating> + Clone,
        worse: impl Iterator<Item = Rating> + Clone,
    ) -> f64 {
        let all = better
            .clone()
            .chain(tied)
            .chain(worse.clone())
            .map(Into::into);
        let pos_offset: f64 = better.map(|rating| rating.sig.recip()).sum();
        let neg_offset: f64 = worse.map(|rating| rating.sig.recip()).sum();
        robust_average(all, pos_offset - neg_offset, 0.)
    }
}

impl RatingSystem for EloRSystem {
    fn win_probability(&self, player: &Rating, foe: &Rating) -> f64 {
        let sigma = (player.sig.powi(2) + foe.sig.powi(2) + 2. * self.sig_perf.powi(2)).sqrt();
        let z = (player.mu - foe.mu) / sigma;
        match self.variant {
            EloRVariant::Gaussian => standard_normal_cdf(z),
            _ => standard_logistic_cdf(z),
        }
    }

    fn round_update(&self, mut standings: Vec<(&mut Player, usize, usize)>) {
        // Update ratings due to waiting period between contests
        let all_ratings: Vec<Rating> = standings
            .par_iter_mut()
            .map(|(player, _, _)| {
                match self.variant {
                    // if transfer_speed is infinite or the system is Gaussian, the logistic
                    // weights become zero so our spacial-case optimization clears them out
                    EloRVariant::Logistic(transfer_speed) if transfer_speed < f64::INFINITY => {
                        player.add_noise_best(self.sig_drift, transfer_speed)
                    }
                    _ => player.add_noise_and_collapse(self.sig_drift),
                }
                let rating = player.approx_posterior;
                Rating {
                    mu: rating.mu,
                    sig: rating.sig.hypot(self.sig_perf),
                }
            })
            .collect();

        // The computational bottleneck: update ratings based on contest performance
        standings
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (player, lo, hi))| {
                let better = all_ratings[..lo].iter().cloned();
                let tied = all_ratings[lo..=hi]
                    .iter()
                    .cloned()
                    .chain(std::iter::once(all_ratings[i]));
                let worse = all_ratings[hi + 1..].iter().cloned();

                let perf = match self.variant {
                    EloRVariant::Gaussian => {
                        Self::compute_performance_gaussian(better, tied, worse)
                    }
                    _ => Self::compute_performance_logistic(better, tied, worse),
                };

                player.update_rating_with_new_performance(
                    Rating {
                        mu: perf,
                        sig: self.sig_perf,
                    },
                    usize::MAX,
                );
            });
    }
}
