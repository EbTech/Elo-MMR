//! Elo-R system details: https://arxiv.org/abs/2101.00400
use super::{Player, Rating, RatingSystem, TanhTerm, SECS_PER_DAY};
use crate::data_processing::ContestRatingParams;
use crate::numerical::{solve_newton, standard_normal_cdf, standard_normal_pdf};
use core::ops::Range;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use superslice::Ext;

type SmallVec = smallvec::SmallVec<[usize; 1]>;
//type SmallVec = Vec<usize>;

trait Term {
    fn eval(&self, x: f64, order: Ordering, split_ties: bool) -> (f64, f64);
    // my_rank is assumed to be a non-empty, sorted slice.
    // This function is a computational bottleneck, so it's important to optimize.
    fn evals(&self, x: f64, ranks: &[usize], my_rank: usize, split_ties: bool) -> (f64, f64) {
        if ranks.len() == 1 {
            // The unit-length case is very common, so we optimize it.
            return self.eval(x, ranks[0].cmp(&my_rank), split_ties);
        }
        let Range { start, end } = ranks.equal_range(&my_rank);
        let equal = end - start;
        let greater = ranks.len() - end;
        let mut value = 0.;
        let mut deriv = 0.;
        if start > 0 {
            let (v, p) = self.eval(x, Ordering::Less, split_ties);
            value += start as f64 * v;
            deriv += start as f64 * p;
        }
        if equal > 0 {
            let (v, p) = self.eval(x, Ordering::Equal, split_ties);
            value += equal as f64 * v;
            deriv += equal as f64 * p;
        }
        if greater > 0 {
            let (v, p) = self.eval(x, Ordering::Greater, split_ties);
            value += greater as f64 * v;
            deriv += greater as f64 * p;
        }
        (value, deriv)
    }
}

impl Term for Rating {
    fn eval(&self, x: f64, order: Ordering, split_ties: bool) -> (f64, f64) {
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
                    let cdf_m1 = -standard_normal_cdf(-z);
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
    fn eval(&self, x: f64, order: Ordering, split_ties: bool) -> (f64, f64) {
        let (val, val_prime) = self.base_values(x);
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

    // Override to optimize this case where win, loss, and draw terms are similar.
    fn evals(&self, x: f64, ranks: &[usize], my_rank: usize, split_ties: bool) -> (f64, f64) {
        if ranks.len() == 1 {
            // The unit-length case is very common, so we optimize it.
            return self.eval(x, ranks[0].cmp(&my_rank), split_ties);
        }
        let (val, val_prime) = self.base_values(x);
        let Range { start, end } = ranks.equal_range(&my_rank);
        let mut total = ranks.len() as f64;
        let win_minus_loss = total - (start + end) as f64;
        if !split_ties {
            let equal = end - start;
            total += equal as f64;
        }

        let value = val * total + self.w_out * win_minus_loss;
        let deriv = val_prime * total;
        (value, deriv)
    }
}

fn bucket(a: f64, width: f64) -> i32 {
    (a / width).round() as i32
}

fn same_bucket(a: f64, b: f64, width: f64) -> bool {
    bucket(a, width) == bucket(b, width)
}

fn cmp_by_bucket(a: f64, b: f64, width: f64) -> Ordering {
    bucket(a, width).cmp(&bucket(b, width))
}

#[derive(Debug)]
pub enum EloMMRVariant {
    Gaussian,
    Logistic(f64),
}

#[derive(Debug)]
pub struct EloMMR {
    // the weight of each new contest
    pub weight_limit: f64,
    // weight multipliers (less than one) to apply on first few contests
    pub noob_delay: Vec<f64>,
    // each contest participation adds an amount of drift such that, in the absence of
    // much time passing, the limiting skill uncertainty's square approaches this value
    pub sig_limit: f64,
    // additional variance per day, from a drift that's continuous in time
    pub drift_per_day: f64,
    // whether to count ties as half a win plus half a loss
    pub split_ties: bool,
    // maximum number of opponents and recent events to use, as a compute-saving approximation
    pub subsample_size: usize,
    // width of mu and sigma to group subsamples by
    pub subsample_bucket: f64,
    // whether to use a Gaussian or logistic performance model
    pub variant: EloMMRVariant,
}

impl Default for EloMMR {
    fn default() -> Self {
        Self::from_limit(0.2, 80., false, false, EloMMRVariant::Logistic(1.))
    }
}

impl EloMMR {
    pub fn default_fast() -> Self {
        Self::from_limit(0.2, 80., false, true, EloMMRVariant::Logistic(1.))
    }

    pub fn default_gaussian() -> Self {
        Self::from_limit(0.2, 80., false, false, EloMMRVariant::Gaussian)
    }

    pub fn default_gaussian_fast() -> Self {
        Self::from_limit(0.2, 80., false, true, EloMMRVariant::Gaussian)
    }

    // beta must exceed sig_limit, the limiting uncertainty for a player with long history
    // the ratio (sig_limit / beta) effectively determines the rating update weight
    pub fn from_limit(
        weight_limit: f64,
        sig_limit: f64,
        split_ties: bool,
        fast: bool,
        variant: EloMMRVariant,
    ) -> Self {
        assert!(weight_limit > 0.);
        assert!(sig_limit > 0.);
        let noob_delay = vec![];
        let subsample_size = if fast { 100 } else { usize::MAX };
        let subsample_bucket = if fast { 2. } else { 1e-5 };
        Self {
            weight_limit,
            noob_delay,
            sig_limit,
            drift_per_day: 0.,
            split_ties,
            subsample_size,
            subsample_bucket,
            variant,
        }
    }

    fn compute_weight(&self, mut contest_weight: f64, n: usize) -> f64 {
        contest_weight *= self.weight_limit;
        if let Some(delay_factor) = self.noob_delay.get(n) {
            contest_weight *= delay_factor;
        }
        contest_weight
    }

    fn compute_sig_perf(&self, weight: f64) -> f64 {
        let discrete_perf = (1. + 1. / weight) * self.sig_limit * self.sig_limit;
        let continuous_perf = self.drift_per_day / weight;
        (discrete_perf + continuous_perf).sqrt()
    }

    fn compute_sig_drift(&self, weight: f64, delta_secs: f64) -> f64 {
        let discrete_drift = weight * self.sig_limit * self.sig_limit;
        let continuous_drift = self.drift_per_day * delta_secs / SECS_PER_DAY;
        (discrete_drift + continuous_drift).sqrt()
    }

    fn subsample(
        terms: &[(Rating, SmallVec)],
        rating: f64,
        num_samples: usize,
        subsample_bucket: f64,
    ) -> impl Iterator<Item = usize> + Clone {
        // TODO: ensure the player still includes themself exactly once
        let mut beg = terms
            .binary_search_by(|term| {
                cmp_by_bucket(term.0.mu, rating, subsample_bucket).then(std::cmp::Ordering::Greater)
            })
            .unwrap_err();
        let mut end = beg + 1;

        let expand = (num_samples.saturating_sub(end - beg) + 1) / 2;
        beg = beg.saturating_sub(expand);
        end = terms.len().min(end + expand);

        let expand = num_samples.saturating_sub(end - beg);
        beg = beg.saturating_sub(expand);
        end = terms.len().min(end + expand);

        beg..end
        //.filter(move |&i| i != player_i)
        //.chain(Some(player_i))
    }
}

impl RatingSystem for EloMMR {
    fn round_update(
        &self,
        params: ContestRatingParams,
        mut standings: Vec<(&mut Player, usize, usize)>,
    ) {
        // Update ratings due to waiting period between contests,
        // then use it to create Gaussian terms for the Q-function.
        // The rank must also be stored in order to determine if it's a win, loss, or tie
        // term. filter_map can exclude the least useful terms from subsampling.
        let mut base_terms: Vec<(Rating, usize)> = standings
            .par_iter_mut()
            .map(|(player, lo, _)| {
                let weight = self.compute_weight(params.weight, player.times_played_excl());
                let sig_perf = self.compute_sig_perf(weight);
                let sig_drift = self.compute_sig_drift(weight, player.delta_time as f64);
                match self.variant {
                    // if transfer_speed is infinite or the prior is Gaussian, the logistic
                    // weights become zero so this special-case optimization clears them out
                    EloMMRVariant::Logistic(transfer_speed) if transfer_speed < f64::INFINITY => {
                        player.add_noise_best(sig_drift, transfer_speed)
                    }
                    _ => player.add_noise_and_collapse(sig_drift),
                }
                (player.approx_posterior.with_noise(sig_perf), *lo)
            })
            .collect();

        // Sort terms by rating to allow for subsampling within a range or ratings.
        base_terms.sort_unstable_by(|a, b| {
            cmp_by_bucket(a.0.mu, b.0.mu, self.subsample_bucket)
                .then_with(|| cmp_by_bucket(a.0.sig, b.0.sig, self.subsample_bucket))
                .then_with(|| a.1.cmp(&b.1))
        });
        let mut normal_terms: Vec<(Rating, SmallVec)> = vec![];
        for (term, lo) in base_terms {
            if let Some((last_term, ranks)) = normal_terms.last_mut() {
                if same_bucket(last_term.mu, term.mu, self.subsample_bucket)
                    && same_bucket(last_term.sig, term.sig, self.subsample_bucket)
                {
                    let len = ranks.len() as f64;
                    last_term.mu = (len * last_term.mu + term.mu) / (len + 1.);
                    last_term.sig = (len * last_term.sig + term.sig) / (len + 1.);
                    ranks.push(lo);
                    continue;
                }
            }
            normal_terms.push((term, smallvec::smallvec![lo]));
            //normal_terms.push((term, vec![lo]));
        }

        // Create the equivalent logistic terms.
        let tanh_terms: Vec<(TanhTerm, SmallVec)> = normal_terms
            .iter()
            .map(|(rating, ranks)| ((*rating).into(), ranks.clone()))
            .collect();

        // Store the maximum subsample we've seen so far, to avoid logging excessive warnings
        let idx_len_max = AtomicUsize::new(9999);

        // The computational bottleneck: update ratings based on contest performance
        standings.into_par_iter().for_each(|(player, my_rank, _)| {
            let player_mu = player.approx_posterior.mu;
            let idx_subsample = Self::subsample(
                &normal_terms,
                player_mu,
                self.subsample_size,
                self.subsample_bucket,
            );
            // Log a warning if the subsample size is very large
            let idx_len_upper_bound = idx_subsample.size_hint().1.unwrap_or(usize::MAX);
            if idx_len_max.fetch_max(idx_len_upper_bound, Relaxed) < idx_len_upper_bound {
                tracing::warn!(
                    "Subsampling {} opponents might be slow; consider decreasing subsample_size.",
                    idx_len_upper_bound
                );
            }
            let bounds = (-6000.0, 9000.0);
            let weight = self.compute_weight(params.weight, player.times_played_excl());
            let sig_perf = self.compute_sig_perf(weight);

            match self.variant {
                EloMMRVariant::Gaussian => {
                    let idx_subsample = idx_subsample.map(|i| &normal_terms[i]);
                    let f = |x| {
                        idx_subsample
                            .clone()
                            .map(|(rating, ranks)| rating.evals(x, ranks, my_rank, self.split_ties))
                            .fold((0., 0.), |(s, sp), (v, vp)| (s + v, sp + vp))
                    };
                    let mu_perf = solve_newton(bounds, f);
                    player.update_rating_with_normal(Rating {
                        mu: mu_perf,
                        sig: sig_perf,
                    });
                }
                EloMMRVariant::Logistic(_) => {
                    let idx_subsample = idx_subsample.map(|i| &tanh_terms[i]);
                    let f = |x| {
                        idx_subsample
                            .clone()
                            .map(|(rating, ranks)| rating.evals(x, ranks, my_rank, self.split_ties))
                            .fold((0., 0.), |(s, sp), (v, vp)| (s + v, sp + vp))
                    };
                    let mu_perf = solve_newton(bounds, f).min(params.perf_ceiling);
                    player.update_rating_with_logistic(
                        Rating {
                            mu: mu_perf,
                            sig: sig_perf,
                        },
                        self.subsample_size, // TODO: make separate history length parameter
                    );
                }
            };
        });
    }
}
