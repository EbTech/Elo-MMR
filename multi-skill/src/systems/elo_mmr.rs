//! Elo-R system details: https://arxiv.org/abs/2101.00400
use super::util::{
    solve_newton, standard_normal_cdf, standard_normal_pdf, Player, Rating, RatingSystem, TanhTerm,
};
use rayon::prelude::*;
use std::cmp::Ordering;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

trait Term {
    fn eval(&self, x: f64, order: Ordering, split_ties: bool) -> (f64, f64);
    // my_rank is assumed to be a non-empty, sorted slice.
    // This function is a computational bottleneck, so it's important to optimize.
    fn evals(&self, x: f64, ranks: &[usize], my_rank: usize, split_ties: bool) -> (f64, f64) {
        if ranks.len() == 1 {
            // The unit-length case is very common, so we optimize it.
            let (value, deriv) = self.eval(x, ranks[0].cmp(&my_rank), split_ties);
            return (value, deriv);
        }
        let (pref, suff, mut value, mut deriv) = match ranks.binary_search(&my_rank) {
            Ok(pref) => {
                let suff = ranks.len() - pref - 1;
                let (e, ep) = self.eval(x, Ordering::Equal, split_ties);
                (pref, suff, e, ep)
            }
            Err(pref) => {
                let suff = ranks.len() - pref;
                (pref, suff, 0., 0.)
            }
        };
        if pref > 0 {
            let (l, lp) = self.eval(x, Ordering::Less, split_ties);
            value += pref as f64 * l;
            deriv += pref as f64 * lp;
        }
        if suff > 0 {
            let (g, gp) = self.eval(x, Ordering::Greater, split_ties);
            value += suff as f64 * g;
            deriv += suff as f64 * gp;
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
    fn eval(&self, x: f64, order: Ordering, split_ties: bool) -> (f64, f64) {
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

fn bucket(a: f64, width: f64) -> i32 {
    (a / width).round() as i32
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
    // squared variation in individual performances, when the contest_weight is 1
    pub beta: f64,
    // each contest participation adds an amount of drift such that, in the absence of
    // much time passing, the limiting skill uncertainty's square approaches this value
    pub sig_limit: f64,
    // additional variance per second, from a drift that's continuous in time
    pub drift_per_sec: f64,
    // whether to count ties as half a win plus half a loss
    pub split_ties: bool,
    // maximum number of opponents and recent events to use, as a compute-saving approximation
    pub subsample_size: usize,
    // whether to use a Gaussian or logistic performance model
    pub variant: EloMMRVariant,
}

impl Default for EloMMR {
    fn default() -> Self {
        Self::from_limit(200., 80., false, false, EloMMRVariant::Logistic(1.))
    }
}

impl EloMMR {
    pub fn default_fast() -> Self {
        Self::from_limit(200., 80., false, true, EloMMRVariant::Logistic(1.))
    }

    pub fn default_gaussian() -> Self {
        Self::from_limit(200., 80., false, false, EloMMRVariant::Gaussian)
    }

    pub fn default_gaussian_fast() -> Self {
        Self::from_limit(200., 80., false, true, EloMMRVariant::Gaussian)
    }

    // sig_perf must exceed sig_limit, the limiting uncertainty for a player with long history
    // the ratio (sig_limit / sig_perf) effectively determines the rating update weight
    pub fn from_limit(
        beta: f64,
        sig_limit: f64,
        split_ties: bool,
        fast: bool,
        variant: EloMMRVariant,
    ) -> Self {
        assert!(sig_limit > 0.);
        assert!(beta > sig_limit);
        let subsample_size = if fast { 100 } else { usize::MAX };
        Self {
            beta,
            sig_limit,
            drift_per_sec: 0.,
            split_ties,
            subsample_size,
            variant,
        }
    }

    fn sig_perf_and_drift(&self, contest_weight: f64) -> (f64, f64) {
        let excess_beta_sq =
            (self.beta * self.beta - self.sig_limit * self.sig_limit) / contest_weight;
        let sig_perf = (self.sig_limit * self.sig_limit + excess_beta_sq).sqrt();
        let discrete_drift = self.sig_limit.powi(4) / excess_beta_sq;
        (sig_perf, discrete_drift)
    }

    fn subsample(
        terms: &[(Rating, Vec<usize>)],
        rating: f64,
        num_samples: usize,
    ) -> impl Iterator<Item = usize> + Clone {
        // TODO: ensure the player still includes themself exactly once
        let mut beg = terms
            .binary_search_by(|term| {
                cmp_by_bucket(term.0.mu, rating, 2.).then(std::cmp::Ordering::Greater)
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
    fn round_update(&self, contest_weight: f64, mut standings: Vec<(&mut Player, usize, usize)>) {
        let (sig_perf, discrete_drift) = self.sig_perf_and_drift(contest_weight);

        // Update ratings due to waiting period between contests,
        // then use it to create Gaussian terms for the Q-function.
        // The rank must also be stored in order to determine if it's a win, loss, or tie
        // term. filter_map can exclude the least useful terms from subsampling.
        let mut base_terms: Vec<(Rating, usize)> = standings
            .par_iter_mut()
            .map(|(player, lo, _)| {
                let continuous_drift = self.drift_per_sec * player.update_time as f64;
                let sig_drift = (discrete_drift + continuous_drift).sqrt();
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
            cmp_by_bucket(a.0.mu, b.0.mu, 2.)
                .then_with(|| cmp_by_bucket(a.0.sig, b.0.sig, 2.))
                .then_with(|| a.1.cmp(&b.1))
        });
        let mut normal_terms: Vec<(Rating, Vec<usize>)> = vec![];
        for (term, lo) in base_terms {
            if let Some((last_term, ranks)) = normal_terms.last_mut() {
                let len = ranks.len() as f64;
                if bucket(last_term.mu, 2.) == bucket(term.mu, 2.)
                    && bucket(last_term.sig, 2.) == bucket(term.sig, 2.)
                {
                    last_term.mu = (len * last_term.mu + term.mu) / (len + 1.);
                    last_term.sig = (len * last_term.sig + term.sig) / (len + 1.);
                    ranks.push(lo);
                    continue;
                }
            }
            normal_terms.push((term, vec![lo]));
        }

        // Create the equivalent logistic terms.
        let tanh_terms: Vec<(TanhTerm, Vec<usize>)> = normal_terms
            .iter()
            .map(|(rating, ranks)| ((*rating).into(), ranks.clone()))
            .collect();

        // Store the maximum subsample we've seen so far, to avoid logging excessive warnings
        let idx_len_max = AtomicUsize::new(9999);

        // The computational bottleneck: update ratings based on contest performance
        standings.into_par_iter().for_each(|(player, my_rank, _)| {
            /*let extra = if elim_newcomers && player.is_newcomer() {
                Some(player.approx_posterior.with_noise(sig_perf))
            } else {
                None
            };*/
            let player_mu = player.approx_posterior.mu;
            let idx_subsample =
                Self::subsample(&normal_terms, player_mu, self.subsample_size);
            // Log a warning if the subsample size is very large
            let idx_len_upper_bound = idx_subsample.size_hint().1.unwrap_or(usize::MAX);
            if idx_len_max.fetch_max(idx_len_upper_bound, Relaxed) < idx_len_upper_bound {
                eprintln!(
                    "WARNING: subsampling {} opponents might be slow; consider decreasing subsample_size.",
                    idx_len_upper_bound
                );
            }
            let bounds = (-6000.0, 9000.0);

            match self.variant {
                EloMMRVariant::Gaussian => {
                    let idx_subsample = idx_subsample
                        .map(|i| &normal_terms[i]);
                        //.chain(extra.map(|rating| (rating, my_rank)));
                    let f = |x| {
                        idx_subsample
                            .clone()
                            .map(|(rating, ranks)| {
                                rating.evals(x, &ranks, my_rank, self.split_ties)
                            })
                            .fold((0., 0.), |(s, sp), (v, vp)| (s + v, sp + vp))
                    };
                    let mu_perf = solve_newton(bounds, f);
                    player.update_rating_with_normal(Rating {
                        mu: mu_perf,
                        sig: sig_perf,
                    });
                }
                EloMMRVariant::Logistic(_) => {
                    let idx_subsample = idx_subsample
                        .map(|i| &tanh_terms[i]);
                        //.chain(extra.map(|rating| (rating.into(), my_rank)));
                    let f = |x| {
                        idx_subsample
                            .clone()
                            .map(|(rating, ranks)| {
                                rating.evals(x, &ranks, my_rank, self.split_ties)
                            })
                            .fold((0., 0.), |(s, sp), (v, vp)| (s + v, sp + vp))
                    };
                    let mu_perf = solve_newton(bounds, f);
                    player.update_rating_with_logistic(
                        Rating {
                            mu: mu_perf,
                            sig: sig_perf,
                        },
                        self.subsample_size,
                    );
                }
            };
        });
    }
}
