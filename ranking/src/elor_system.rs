//! Elo-R system details: https://github.com/EbTech/EloR/blob/master/paper/EloR.pdf

use crate::compute_ratings::{
    standard_logistic_cdf, standard_normal_cdf, standard_normal_pdf, Player, Rating, RatingSystem,
    TanhTerm,
};
use rayon::prelude::*;
use std::cmp::Ordering;

trait Term {
    fn eval(self, x: f64, order: Ordering, split_ties: bool) -> (f64, f64);
}

impl Term for Rating {
    fn eval(self, x: f64, order: Ordering, split_ties: bool) -> (f64, f64) {
        let z = (x - self.mu) / self.sig;
        let pdf = standard_normal_pdf(z) / self.sig;
        let pdf_prime = -z * pdf / self.sig;

        match order {
            Ordering::Less => {
                // -cdf(-z) is a numerically stable version of cdf(z)-1
                let cdf_m1 = -standard_normal_cdf(-z);
                let val = pdf / cdf_m1;
                (val, pdf_prime / cdf_m1 - val * val)
            }
            Ordering::Greater => {
                let cdf = standard_normal_cdf(z);
                let val = pdf / cdf;
                (val, pdf_prime / cdf - val * val)
            }
            Ordering::Equal => {
                if split_ties {
                    let cdf = standard_normal_cdf(z);
                    let cdf_m1 = cdf - 1.;
                    let val0 = pdf / cdf;
                    let val1 = pdf / cdf_m1;
                    (
                        0.5 * (val0 + val1),
                        0.5 * (pdf_prime * (1. / cdf + 1. / cdf_m1) - val0 * val0 - val1 * val1),
                    )
                } else {
                    let pdf_pp = -(pdf / self.sig + z * pdf_prime) / self.sig;
                    let val = pdf_prime / pdf;
                    (val, pdf_pp / pdf - val * val)
                }
            }
        }
    }
}

impl Term for TanhTerm {
    fn eval(self, x: f64, order: Ordering, split_ties: bool) -> (f64, f64) {
        let z = (x - self.mu) * self.w_arg;
        let val = -z.tanh() * self.w_out;
        let val_prime = -z.cosh().powi(-2) * self.w_arg * self.w_out;

        match order {
            Ordering::Less => (val - self.w_out, val_prime),
            Ordering::Greater => (val + self.w_out, val_prime),
            Ordering::Equal => {
                if split_ties {
                    (val, val_prime)
                } else {
                    (2. * val, 2. * val_prime)
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum EloRVariant {
    Gaussian,
    Logistic(f64),
}

#[derive(Debug)]
pub struct EloRSystem {
    pub sig_perf: f64,        // variation in individual performances
    pub sig_drift: f64,       // skill drift between successive performances
    pub split_ties: bool,     // whether to split ties into half win and half loss
    pub variant: EloRVariant, // whether to use logistic or Gaussian distributions
}

impl Default for EloRSystem {
    fn default() -> Self {
        Self::from_limit(200., 80., false, EloRVariant::Logistic(1.))
    }
}

impl EloRSystem {
    // sig_perf must exceed sig_limit, the limiting uncertainty for a player with long history
    // the ratio (sig_limit / sig_perf) effectively determines the rating update weight
    pub fn from_limit(
        sig_perf: f64,
        sig_limit: f64,
        split_ties: bool,
        variant: EloRVariant,
    ) -> Self {
        assert!(sig_limit > 0.);
        assert!(sig_perf > sig_limit);
        let sig_drift =
            ((sig_limit.powi(-2) - sig_perf.powi(-2)).recip() - sig_limit.powi(2)).sqrt();
        Self {
            sig_perf,
            sig_drift,
            split_ties,
            variant,
        }
    }

    fn newton_step(
        all: impl Iterator<Item = (impl Term, usize)>,
        my_rank: usize,
        split_ties: bool,
        guess: f64,
    ) -> (f64, f64) {
        let mut sum = 0.;
        let mut sum_prime = 0.;
        for (term, rank) in all {
            let order = rank.cmp(&my_rank);
            let (sum_incr, sum_prime_incr) = term.eval(guess, order, split_ties);

            sum += sum_incr;
            sum_prime += sum_prime_incr;
        }
        let next = guess - sum / sum_prime;
        (next, sum)
    }

    fn compute_performance_newton(
        all: impl Iterator<Item = (impl Term, usize)> + Clone,
        my_rank: usize,
        split_ties: bool,
    ) -> f64 {
        let (mut lo, mut hi) = (-6000.0, 9000.0);
        let mut guess = 0.5 * (lo + hi);
        loop {
            let (next, sum) = Self::newton_step(all.clone(), my_rank, split_ties, guess);
            if next < guess {
                hi = guess;
            } else {
                lo = guess;
            }

            if sum.abs() < 1e-10 {
                return next;
            }
            if hi - lo < 1e-12 {
                eprintln!(
                    "WARNING: POSSIBLE FAILURE TO CONVERGE: {}->{} s={}",
                    guess, next, sum
                );
                return next;
            }
            guess = next
                .max(0.75 * lo + 0.25 * guess)
                .min(0.25 * guess + 0.75 * hi);
        }
    }

    fn compute_performance_bsearch(
        all: impl Iterator<Item = (impl Term, usize)> + Clone,
        my_rank: usize,
        split_ties: bool,
    ) -> f64 {
        let (mut lo, mut hi) = (-6000.0, 9000.0);
        while hi - lo > 1e-12 {
            let guess = 0.5 * (lo + hi);
            let mut sum = 0.;
            for (rating, rank) in all.clone() {
                sum += rating.eval(guess, rank.cmp(&my_rank), split_ties).0;
            }

            if sum < 0.0 {
                hi = guess;
            } else {
                lo = guess;
            }
        }
        0.5 * (lo + hi)
    }
}

impl RatingSystem for EloRSystem {
    fn win_probability(&self, player: &Rating, foe: &Rating) -> f64 {
        let sigma = (player.sig.powi(2) + foe.sig.powi(2) + 2. * self.sig_perf.powi(2)).sqrt();
        let z = (player.mu - foe.mu) / sigma;
        match self.variant {
            EloRVariant::Gaussian => standard_normal_cdf(z),
            EloRVariant::Logistic(_) => standard_logistic_cdf(z),
        }
    }

    fn round_update(&self, mut standings: Vec<(&mut Player, usize, usize)>) {
        // Update ratings due to waiting period between contests
        let all_ratings: Vec<(Rating, usize)> = standings
            .par_iter_mut()
            .map(|(player, lo, _)| {
                match self.variant {
                    // if transfer_speed is infinite or the system is Gaussian, the logistic
                    // weights become zero so this special-case optimization clears them out
                    EloRVariant::Logistic(transfer_speed) if transfer_speed < f64::INFINITY => {
                        player.add_noise_best(self.sig_drift, transfer_speed)
                    }
                    _ => player.add_noise_and_collapse(self.sig_drift),
                }
                (player.approx_posterior.with_noise(self.sig_perf), *lo)
            })
            .collect();

        let tanh_terms: Vec<(TanhTerm, usize)> = all_ratings
            .iter()
            .map(|&(rating, lo)| (rating.into(), lo))
            .collect();

        let mut idx_by_rating: Vec<usize> = (0..all_ratings.len()).collect();
        idx_by_rating.sort_unstable_by(|&i, &j| {
            all_ratings[i]
                .0
                .mu
                .partial_cmp(&all_ratings[j].0.mu)
                .unwrap()
        });

        // The computational bottleneck: update ratings based on contest performance
        standings
            .into_par_iter()
            .enumerate()
            .for_each(|(player_i, (player, rank, _))| {
                const NUM_RECENTER: usize = 1;
                const WIDTH_SUBSAMPLE: usize = 1_000_000;

                let mut guess = player.approx_posterior.mu;
                let (mut lo, mut hi) = (-6000., 9000.);
                let (mut iter, mut beg, mut end) = (0, 0, 0);
                let perf = loop {
                    if iter < NUM_RECENTER {
                        iter += 1;
                        beg = idx_by_rating
                            .binary_search_by(|&i| {
                                all_ratings[i]
                                    .0
                                    .mu
                                    .partial_cmp(&guess)
                                    .unwrap()
                                    .then(std::cmp::Ordering::Greater)
                            })
                            .unwrap_err();
                        end = idx_by_rating
                            .binary_search_by(|&i| {
                                all_ratings[i]
                                    .0
                                    .mu
                                    .partial_cmp(&guess)
                                    .unwrap()
                                    .then(std::cmp::Ordering::Less)
                            })
                            .unwrap_err();

                        let expand = (WIDTH_SUBSAMPLE.saturating_sub(end - beg) + 1) / 2;
                        beg = beg.saturating_sub(expand);
                        end = all_ratings.len().min(end + expand);

                        let expand = WIDTH_SUBSAMPLE.saturating_sub(end - beg);
                        beg = beg.saturating_sub(expand);
                        end = all_ratings.len().min(end + expand);

                        beg = idx_by_rating
                            .binary_search_by(|&i| {
                                all_ratings[i]
                                    .0
                                    .mu
                                    .partial_cmp(&all_ratings[idx_by_rating[beg]].0.mu)
                                    .unwrap()
                                    .then(std::cmp::Ordering::Greater)
                            })
                            .unwrap_err();
                        end = idx_by_rating
                            .binary_search_by(|&i| {
                                all_ratings[i]
                                    .0
                                    .mu
                                    .partial_cmp(&all_ratings[idx_by_rating[end - 1]].0.mu)
                                    .unwrap()
                                    .then(std::cmp::Ordering::Less)
                            })
                            .unwrap_err();
                    }
                    let all = idx_by_rating[beg..end]
                        .iter()
                        .cloned()
                        .filter(|&i| i != player_i)
                        .chain(Some(player_i));

                    let (next, sum) = match self.variant {
                        EloRVariant::Gaussian => Self::newton_step(
                            all.clone().map(|i| all_ratings[i]),
                            rank,
                            self.split_ties,
                            guess,
                        ),
                        EloRVariant::Logistic(_) => Self::newton_step(
                            all.clone().map(|i| tanh_terms[i]),
                            rank,
                            self.split_ties,
                            guess,
                        ),
                    };
                    if iter >= NUM_RECENTER {
                        if next < guess {
                            hi = guess;
                        } else {
                            lo = guess;
                        }
                    }

                    if sum.abs() < 1e-10 {
                        break next;
                    }
                    if hi - lo < 1e-12 {
                        eprintln!(
                            "WARNING: POSSIBLE FAILURE TO CONVERGE: {}->{} s={}",
                            guess, next, sum
                        );
                        break next;
                    }
                    guess = next
                        .max(0.75 * lo + 0.25 * guess)
                        .min(0.25 * guess + 0.75 * hi);
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
