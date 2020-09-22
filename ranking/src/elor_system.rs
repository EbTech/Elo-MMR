use super::contest_config::Contest;
use super::compute_ratings::{RatingSystem, Rating, Player, robust_average};
use rayon::prelude::*;
use std::cell::{RefCell, RefMut};
use std::cmp::max;
use std::collections::{HashMap, VecDeque};

/// Elo-R system details: https://github.com/EbTech/EloR/blob/master/paper/EloR.pdf
pub struct EloRSystem {
    sig_perf: f64,  // variation in individual performances
    sig_limit: f64, // limiting uncertainty for a player who competed a lot
}

impl Default for EloRSystem {
    fn default() -> Self {
        Self {
            sig_perf: 250.,
            sig_limit: 100.,
        }
    }
}

impl EloRSystem {
    // Given the participants which beat us, tied with us, and lost against us,
    // returns our Gaussian-weighted performance score for this round
    fn compute_performance_gaussian(
        better: impl Iterator<Item = Rating> + Clone,
        tied: impl Iterator<Item = Rating> + Clone,
        worse: impl Iterator<Item = Rating> + Clone,
    ) -> f64 {
        use statrs::function::erf::erf;
        use std::f64::consts::{PI, SQRT_2};
        let sqrt_2_pi = (2. * PI).sqrt();

        // This is a slow binary search, without Newton steps
        let (mut lo, mut hi) = (-1000.0, 4500.0);
        while hi - lo > 1e-9 {
            let guess = 0.5 * (lo + hi);
            let mut sum = 0.;
            for rating in better.clone() {
                let z = (guess - rating.mu) / rating.sig;
                let pdf = (-0.5 * z * z).exp() / rating.sig / sqrt_2_pi;
                let cdf = 0.5 * (1. + erf(z / SQRT_2));
                sum += pdf / (cdf - 1.);
            }
            for rating in tied.clone() {
                let z = (guess - rating.mu) / rating.sig;
                let pdf = (-0.5 * z * z).exp() / rating.sig / sqrt_2_pi;
                let pdf_prime = -z * pdf / rating.sig;
                sum += pdf_prime / pdf;
            }
            for rating in worse.clone() {
                let z = (guess - rating.mu) / rating.sig;
                let pdf = (-0.5 * z * z).exp() / rating.sig / sqrt_2_pi;
                let cdf = 0.5 * (1. + erf(z / SQRT_2));
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
        let all = better.clone().chain(tied).chain(worse.clone());
        let pos_offset: f64 = better.map(|rating| rating.sig.recip()).sum();
        let neg_offset: f64 = worse.map(|rating| rating.sig.recip()).sum();
        robust_average(all, pos_offset - neg_offset, 0.)
    }
}

impl RatingSystem for EloRSystem {
    fn round_update(&self, mut standings: Vec<(&mut Player, usize, usize)>) {
        let sig_noise = ((self.sig_limit.powi(-2) - self.sig_perf.powi(-2)).recip()
            - self.sig_limit.powi(2))
        .sqrt();

        // Update ratings due to waiting period between contests
        let all_ratings: Vec<Rating> = standings
            .par_iter_mut()
            .map(|(player, _, _)| {
                player.add_noise_and_collapse(sig_noise);
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
                let perf = Self::compute_performance_logistic(
                    all_ratings[..lo].iter().cloned(),
                    all_ratings[lo..=hi]
                        .iter()
                        .cloned()
                        .chain(std::iter::once(all_ratings[i])),
                    all_ratings[hi + 1..].iter().cloned(),
                );
                player.push_performance(Rating {
                    mu: perf,
                    sig: self.sig_perf,
                });
                player.recompute_posterior();
            });
    }
}

