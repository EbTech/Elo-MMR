use super::{robust_average, Rating, TanhTerm};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlayerEvent {
    pub contest_index: usize,
    pub rating_mu: i32,
    pub rating_sig: i32,
    pub perf_score: i32,
    pub place: usize,
}

impl PlayerEvent {
    pub fn get_display_rating(&self) -> i32 {
        // TODO: get rid of the magic numbers 3 and 80!
        //       3 is a conservative number of stdevs: use 0 to get mean estimates
        //       80 is Elo-MMR's default sig_lim
        self.rating_mu - 3 * (self.rating_sig - 80)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    normal_factor: Rating,
    logistic_factors: VecDeque<TanhTerm>,
    pub event_history: Vec<PlayerEvent>,
    pub approx_posterior: Rating,
    pub update_time: u64,
    pub delta_time: u64,
}

impl Player {
    pub fn with_rating(mu: f64, sig: f64, update_time: u64) -> Self {
        Player {
            normal_factor: Rating { mu, sig },
            logistic_factors: VecDeque::new(),
            event_history: vec![],
            approx_posterior: Rating { mu, sig },
            update_time,
            delta_time: 0,
        }
    }

    pub fn is_newcomer(&self) -> bool {
        self.event_history.len() <= 1
    }

    pub fn update_rating(&mut self, rating: Rating, performance_score: f64) {
        // Assumes that a placeholder history item has been pushed containing contest id and time
        let last_event = self.event_history.last_mut().unwrap();
        assert_eq!(last_event.rating_mu, 0);
        assert_eq!(last_event.rating_sig, 0);
        assert_eq!(last_event.perf_score, 0);

        self.approx_posterior = rating;
        last_event.rating_mu = rating.mu.round() as i32;
        last_event.rating_sig = rating.sig.round() as i32;
        last_event.perf_score = performance_score.round() as i32;
    }

    pub fn update_rating_with_normal(&mut self, performance: Rating) {
        let wn = self.normal_factor.sig.powi(-2);
        let wp = performance.sig.powi(-2);
        self.normal_factor.mu = (wn * self.normal_factor.mu + wp * performance.mu) / (wn + wp);
        self.normal_factor.sig = (wn + wp).recip().sqrt();

        let new_rating = if self.logistic_factors.is_empty() {
            self.normal_factor
        } else {
            self.approximate_posterior(performance.sig)
        };
        self.update_rating(new_rating, performance.mu);
    }

    pub fn update_rating_with_logistic(&mut self, performance: Rating, max_history: usize) {
        if self.logistic_factors.len() >= max_history {
            // wl can be chosen so as to preserve total weight or rating; we choose the former.
            // Either way, the deleted element should be small enough not to matter.
            let logistic = self.logistic_factors.pop_front().unwrap();
            let wn = self.normal_factor.sig.powi(-2);
            let wl = logistic.get_weight();
            self.normal_factor.mu = (wn * self.normal_factor.mu + wl * logistic.mu) / (wn + wl);
            self.normal_factor.sig = (wn + wl).recip().sqrt();
        }
        self.logistic_factors.push_back(performance.into());

        let new_rating = self.approximate_posterior(performance.sig);
        self.update_rating(new_rating, performance.mu);
    }

    // Helper function that assumes the factors have been updated with the latest performance,
    // but self.approx_posterior has not yet been updated with this performance.
    fn approximate_posterior(&self, perf_sig: f64) -> Rating {
        let normal_weight = self.normal_factor.sig.powi(-2);
        let mu = robust_average(
            self.logistic_factors.iter().cloned(),
            -self.normal_factor.mu * normal_weight,
            normal_weight,
        );
        let sig = (self.approx_posterior.sig.powi(-2) + perf_sig.powi(-2))
            .recip()
            .sqrt();
        Rating { mu, sig }
    }

    // Method #1: the Gaussian/Brownian approximation, in which rating is a Markov state
    // Equivalent to method #5 with transfer_speed == f64::INFINITY
    pub fn add_noise_and_collapse(&mut self, sig_noise: f64) {
        self.approx_posterior = self.approx_posterior.with_noise(sig_noise);
        self.normal_factor = self.approx_posterior;
        self.logistic_factors.clear();
    }

    // Method #2: decrease weights without changing logistic sigmas
    // Equivalent to method #5 with transfer_speed == 0
    #[allow(dead_code)]
    pub fn add_noise_in_front(&mut self, sig_noise: f64) {
        let decay = 1.0f64.hypot(sig_noise / self.approx_posterior.sig);
        self.approx_posterior.sig *= decay;

        self.normal_factor.sig *= decay;
        for rating in &mut self.logistic_factors {
            rating.w_out /= decay * decay;
        }
    }

    // #5: a general method with the nicest properties, parametrized by transfer_speed >= 0
    // Reduces to method #1 when transfer_speed == f64::INFINITY
    // Reduces to method #2 when transfer_speed == 0
    pub fn add_noise_best(&mut self, sig_noise: f64, transfer_speed: f64) {
        let new_posterior = self.approx_posterior.with_noise(sig_noise);

        let decay = (self.approx_posterior.sig / new_posterior.sig).powi(2);
        let transfer = decay.powf(transfer_speed);
        self.approx_posterior = new_posterior;

        let wt_norm_old = self.normal_factor.sig.powi(-2);
        let wt_from_norm_old = transfer * wt_norm_old;
        let wt_from_transfers = (1. - transfer)
            * (wt_norm_old
                + self
                    .logistic_factors
                    .iter()
                    .map(TanhTerm::get_weight)
                    .sum::<f64>());
        let wt_total = wt_from_norm_old + wt_from_transfers;

        self.normal_factor.mu = (wt_from_norm_old * self.normal_factor.mu
            + wt_from_transfers * self.approx_posterior.mu)
            / wt_total;
        self.normal_factor.sig = (decay * wt_total).recip().sqrt();
        for r in &mut self.logistic_factors {
            r.w_out *= transfer * decay;
        }
    }
}
