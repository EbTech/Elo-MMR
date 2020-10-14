extern crate overload;

use crate::compute_ratings::Rating;
use overload::overload;
use std::fmt;
use std::ops;

pub type ParticipantRatings = [(Rating, usize, usize)];
pub type Metric = Box<dyn Fn(&ParticipantRatings) -> f64>;

// A data structure for storing the various performance metrics we want to analyze
pub struct PerformanceReport {
    pub num_rounds: usize,
    pub summed_metrics: Vec<f64>,
}

impl fmt::Display for PerformanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let averaged_metrics: Vec<f64> = self
            .summed_metrics
            .iter()
            .map(|x| x / self.num_rounds as f64)
            .collect();
        write!(f, "({}, {:?})", self.num_rounds, averaged_metrics)
    }
}

impl PerformanceReport {
    pub fn new(num_metrics: usize) -> Self {
        Self {
            num_rounds: 0,
            summed_metrics: vec![0.; num_metrics],
        }
    }
}

overload!((a: ?PerformanceReport) + (b: ?PerformanceReport) -> PerformanceReport {
    assert_eq!(a.summed_metrics.len(), b.summed_metrics.len());
    let summed_metrics = a.summed_metrics.iter().zip(b.summed_metrics.iter()).map(|(a_metric, b_metric)| a_metric + b_metric).collect();
    PerformanceReport {
        num_rounds: a.num_rounds + b.num_rounds,
        summed_metrics
    }
});

overload!((a: &mut PerformanceReport) += (b: ?PerformanceReport) {
    assert_eq!(a.summed_metrics.len(), b.summed_metrics.len());
    for (a_metric, b_metric) in a.summed_metrics.iter_mut().zip(b.summed_metrics.iter()) {
        *a_metric += b_metric;
    }
    a.num_rounds += b.num_rounds;
});

// Returns only the players whose 0-indexed rank is less than k
// May return more than k players if there are ties
pub fn top_k(standings: &ParticipantRatings, k: usize) -> &ParticipantRatings {
    let idx_first_ge_k = standings
        .binary_search_by(|&(_, lo, _)| lo.cmp(&k).then(std::cmp::Ordering::Greater))
        .unwrap_err();
    &standings[0..idx_first_ge_k]
}

pub fn pairwise_metric(standings: &ParticipantRatings) -> f64 {
    // Compute topk (frac. of inverted pairs) metric
    let mut correct_pairs = 0.;
    let mut total_pairs = 0.;
    for &(loser_rating, loser_lo, _) in standings {
        for &(winner_rating, winner_lo, _) in standings {
            if winner_lo >= loser_lo as usize {
                break;
            }
            if winner_rating.mu > loser_rating.mu {
                correct_pairs += 1.;
            }
            total_pairs += 1.;
        }
    }
    100. * correct_pairs / total_pairs
}

pub fn percentile_distance_metric(standings: &ParticipantRatings) -> f64 {
    // Compute avg percentile distance metric
    let mut standings_by_rating = Vec::from(standings);
    standings_by_rating.sort_by(|a, b| b.0.mu.partial_cmp(&a.0.mu).unwrap());

    let mut sum_error = 0.;
    for (i, &(_, lo, hi)) in standings_by_rating.iter().enumerate() {
        let closest_to_i = i.max(lo).min(hi);
        sum_error += (i as f64 - closest_to_i as f64).abs();
    }
    100. * sum_error / (standings_by_rating.len() as f64).powi(2)
}

pub fn cross_entropy_metric(standings: &ParticipantRatings) -> f64 {
    // Compute cross-entropy from the logistic Elo formula
    let mut sum_ce = 0.;
    let mut total_pairs = 0.;
    for &(loser_rating, loser_lo, _) in standings {
        for &(winner_rating, winner_lo, _) in standings {
            if winner_lo >= loser_lo as usize {
                break;
            }
            let rating_diff = loser_rating.mu - winner_rating.mu;
            let inv_prob = 1. + 10f64.powf(rating_diff / 400.);
            sum_ce += inv_prob.ln();
            total_pairs += 1.;
        }
    }
    sum_ce / total_pairs
}

fn exist_distinct_places(standings: &ParticipantRatings) -> bool {
    if let Some(&(_, last_lo, _)) = standings.last() {
        let (_, first_lo, _) = standings[0];
        first_lo != last_lo
    } else {
        false
    }
}

// Example of how to create the metrics argument:
// let topk_metric: Metric = Box::new(move |s| pairwise_metric(top_k(s, topk)));
// let percent_metric: Metric = Box::new(percentile_distance_metric);
// let metrics = vec![topk_metric, percent_metric]
pub fn compute_metrics_by_fn(
    standings: &ParticipantRatings,
    metrics: &[Metric],
) -> PerformanceReport {
    if !exist_distinct_places(standings) {
        PerformanceReport::new(metrics.len())
    } else {
        PerformanceReport {
            num_rounds: 1,
            summed_metrics: metrics.iter().map(|f| f(standings)).collect(),
        }
    }
}

// Meant to be modified manually to contain the desired metrics
pub fn compute_metrics_custom(standings: &ParticipantRatings, k: usize) -> PerformanceReport {
    if !exist_distinct_places(standings) {
        PerformanceReport::new(3)
    } else {
        let topk = pairwise_metric(top_k(standings, k));
        let percentile = percentile_distance_metric(standings);
        let crossent = cross_entropy_metric(standings);

        PerformanceReport {
            num_rounds: 1,
            summed_metrics: vec![topk, percentile, crossent],
        }
    }
}
