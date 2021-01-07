//! Elo-R system details: https://arxiv.org/abs/2101.00400
use super::util::{
    solve_newton, standard_logistic_cdf, standard_normal_cdf, standard_normal_pdf, Player, Rating,
    RatingSystem, TanhTerm,
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
pub enum EloMMRVariant {
    Gaussian,
    Logistic(f64),
}

#[derive(Debug)]
pub struct EloMMR {
    pub sig_perf: f64,          // variation in individual performances
    pub sig_drift: f64,         // skill drift between successive performances
    pub split_ties: bool,       // whether to split ties into half win and half loss
    pub variant: EloMMRVariant, // whether to use logistic or Gaussian distributions
}

impl Default for EloMMR {
    fn default() -> Self {
        Self::from_limit(200., 80., false, EloMMRVariant::Logistic(1.))
    }
}

impl EloMMR {
    pub fn default_gaussian() -> Self {
        Self::from_limit(200., 80., false, EloMMRVariant::Gaussian)
    }

    // sig_perf must exceed sig_limit, the limiting uncertainty for a player with long history
    // the ratio (sig_limit / sig_perf) effectively determines the rating update weight
    pub fn from_limit(
        sig_perf: f64,
        sig_limit: f64,
        split_ties: bool,
        variant: EloMMRVariant,
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

    fn subsample<'a>(
        idx_by_rating: &'a [usize],
        all_ratings: &[(Rating, usize)],
        rating: f64,
        num_samples: usize,
    ) -> impl Iterator<Item = usize> + Clone + 'a {
        let mut beg = idx_by_rating
            .binary_search_by(|&i| {
                all_ratings[i]
                    .0
                    .mu
                    .partial_cmp(&rating)
                    .unwrap()
                    .then(std::cmp::Ordering::Greater)
            })
            .unwrap_err();
        let mut end = idx_by_rating
            .binary_search_by(|&i| {
                all_ratings[i]
                    .0
                    .mu
                    .partial_cmp(&rating)
                    .unwrap()
                    .then(std::cmp::Ordering::Less)
            })
            .unwrap_err();

        let expand = (num_samples.saturating_sub(end - beg) + 1) / 2;
        beg = beg.saturating_sub(expand);
        end = all_ratings.len().min(end + expand);

        let expand = num_samples.saturating_sub(end - beg);
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
        idx_by_rating[beg..end].iter().cloned()
        //.filter(move |&i| i != player_i)
        //.chain(Some(player_i))
    }
}

impl RatingSystem for EloMMR {
    fn win_probability(&self, player: &Rating, foe: &Rating) -> f64 {
        let sigma = (player.sig.powi(2) + foe.sig.powi(2) + 2. * self.sig_perf.powi(2)).sqrt();
        let z = (player.mu - foe.mu) / sigma;
        match self.variant {
            EloMMRVariant::Gaussian => standard_normal_cdf(z),
            EloMMRVariant::Logistic(_) => standard_logistic_cdf(z),
        }
    }

    fn round_update(&self, mut standings: Vec<(&mut Player, usize, usize)>) {
        const WIDTH_SUBSAMPLE: usize = 2000;
        const MAX_HISTORY_LEN: usize = 500;
        let elim_newcomers = false; /*ignoring newcomers causes severe rating deflation: standings
                                    .par_iter()
                                    .filter(|(player, _, _)| !player.is_newcomer())
                                    .count()
                                    >= WIDTH_SUBSAMPLE;*/

        // Update ratings due to waiting period between contests
        let all_ratings: Vec<(Rating, usize)> = standings
            .par_iter_mut()
            .filter_map(|(player, lo, _)| {
                match self.variant {
                    // if transfer_speed is infinite or the system is Gaussian, the logistic
                    // weights become zero so this special-case optimization clears them out
                    EloMMRVariant::Logistic(transfer_speed) if transfer_speed < f64::INFINITY => {
                        player.add_noise_best(self.sig_drift, transfer_speed)
                    }
                    _ => player.add_noise_and_collapse(self.sig_drift),
                }
                if elim_newcomers && player.is_newcomer() {
                    None
                } else {
                    Some((player.approx_posterior.with_noise(self.sig_perf), *lo))
                }
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
        standings.into_par_iter().for_each(|(player, my_rank, _)| {
            let extra = if elim_newcomers && player.is_newcomer() {
                Some(player.approx_posterior.with_noise(self.sig_perf))
            } else {
                None
            };
            let player_mu = player.approx_posterior.mu;
            let idx_subsample =
                Self::subsample(&idx_by_rating, &all_ratings, player_mu, WIDTH_SUBSAMPLE);
            let bounds = (-6000.0, 9000.0);
            let perf = match self.variant {
                EloMMRVariant::Gaussian => {
                    let idx_subsample = idx_subsample
                        .map(|i| all_ratings[i])
                        .chain(extra.map(|rating| (rating, my_rank)));
                    let f = |x| {
                        idx_subsample
                            .clone()
                            .map(|(rating, rank)| {
                                rating.eval(x, rank.cmp(&my_rank), self.split_ties)
                            })
                            .fold((0., 0.), |(s, sp), (v, vp)| (s + v, sp + vp))
                    };
                    solve_newton(bounds, f)
                }
                EloMMRVariant::Logistic(_) => {
                    let idx_subsample = idx_subsample
                        .map(|i| tanh_terms[i])
                        .chain(extra.map(|rating| (rating.into(), my_rank)));
                    let f = |x| {
                        idx_subsample
                            .clone()
                            .map(|(rating, rank)| {
                                rating.eval(x, rank.cmp(&my_rank), self.split_ties)
                            })
                            .fold((0., 0.), |(s, sp), (v, vp)| (s + v, sp + vp))
                    };
                    solve_newton(bounds, f)
                }
            };

            player.update_rating_with_new_performance(
                Rating {
                    mu: perf,
                    sig: self.sig_perf,
                },
                MAX_HISTORY_LEN,
            );
        });
    }
}
