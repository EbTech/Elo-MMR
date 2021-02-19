use crate::systems::{get_participant_ratings, outcome_free, PlayersByName, Rating};
use overload::overload;
use std::fmt;
use std::ops;

pub type ParticipantRatings = [(Rating, usize, usize)];
pub type WeightAndSum = (f64, f64);
pub type Metric = Box<dyn Fn(&ParticipantRatings) -> f64>;

// A data structure for storing the various performance metrics we want to analyze
pub struct PerformanceReport {
    pub metrics_wt_sum: Vec<WeightAndSum>,
}

impl fmt::Display for PerformanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let averaged: Vec<f64> = self
            .metrics_wt_sum
            .iter()
            .map(|&(wt, sum)| sum / wt)
            .collect();
        write!(f, "{:?})", averaged)
    }
}

impl PerformanceReport {
    pub fn new(num_metrics: usize) -> Self {
        Self {
            metrics_wt_sum: vec![(0., 0.); num_metrics],
        }
    }
}

overload!((a: ?PerformanceReport) + (b: ?PerformanceReport) -> PerformanceReport {
    assert_eq!(a.metrics_wt_sum.len(), b.metrics_wt_sum.len());
    let metrics_wt_sum = a.metrics_wt_sum.iter().zip(b.metrics_wt_sum.iter()).map(|((a_w, a_sum), (b_w, b_sum))| (a_w+b_w, a_sum+b_sum)).collect();
    PerformanceReport {
        metrics_wt_sum
    }
});

overload!((a: &mut PerformanceReport) += (b: ?PerformanceReport) {
    assert_eq!(a.metrics_wt_sum.len(), b.metrics_wt_sum.len());
    for ((a_w, a_sum), (b_w, b_sum)) in a.metrics_wt_sum.iter_mut().zip(b.metrics_wt_sum.iter()) {
        *a_w += b_w;
        *a_sum += b_sum;
    }
});

// Returns only the players whose 0-indexed rank is less than k
// May return more than k players if there are ties
pub fn top_k(standings: &ParticipantRatings, k: usize) -> &ParticipantRatings {
    let idx_first_ge_k = standings
        .binary_search_by(|&(_, lo, _)| lo.cmp(&k).then(std::cmp::Ordering::Greater))
        .unwrap_err();
    &standings[0..idx_first_ge_k]
}

pub fn pairwise_metric(standings: &ParticipantRatings) -> WeightAndSum {
    if outcome_free(standings) {
        return (0., 0.);
    }
    // Compute topk (frac. of inverted pairs) metric
    let mut correct_pairs = 0.;
    let mut total_pairs = 0.;
    for &(loser_rating, loser_lo, _) in standings {
        for &(winner_rating, winner_lo, _) in standings {
            if winner_lo >= loser_lo as usize {
                break;
            }
            if winner_rating.mu > loser_rating.mu {
                correct_pairs += 2.;
            }
            total_pairs += 2.;
        }
    }

    let n = standings.len() as f64;
    let tied_pairs = n * (n - 1.) - total_pairs;
    (n, 100. * (correct_pairs + tied_pairs) / (n - 1.))
}

pub fn percentile_distance_metric(standings: &ParticipantRatings) -> WeightAndSum {
    if outcome_free(standings) {
        return (0., 0.);
    }
    // Compute avg percentile distance metric
    let mut standings_by_rating = Vec::from(standings);
    standings_by_rating.sort_by(|a, b| b.0.mu.partial_cmp(&a.0.mu).unwrap());

    let mut sum_error = 0.;
    for (i, &(_, lo, hi)) in standings_by_rating.iter().enumerate() {
        let closest_to_i = i.clamp(lo, hi);
        sum_error += (i as f64 - closest_to_i as f64).abs();
    }

    let n = standings.len() as f64;
    (n, 100. * sum_error / (n - 1.))
}

pub fn cross_entropy_metric(standings: &ParticipantRatings, scale: f64) -> WeightAndSum {
    if outcome_free(standings) {
        return (0., 0.);
    }
    // Compute base 2 cross-entropy from the logistic Elo formula
    // The default value of scale reported in the paper is 400,
    // all others can be seen as applying to a scaled version of the ratings
    let mut sum_ce = 0.;
    for &(loser_rating, loser_lo, _) in standings {
        for &(winner_rating, winner_lo, _) in standings {
            if winner_lo >= loser_lo as usize {
                break;
            }
            let rating_diff = loser_rating.mu - winner_rating.mu;
            let inv_prob = 1. + 10f64.powf(rating_diff / scale);
            sum_ce += inv_prob.log2();
        }
    }

    let n = standings.len() as f64;
    (n, 2. * sum_ce / (n - 1.))
}

// Meant to be modified manually to contain the desired metrics
pub fn compute_metrics_custom(
    players: &mut PlayersByName,
    contest_standings: &[(String, usize, usize)],
) -> PerformanceReport {
    let everyone = get_participant_ratings(players, contest_standings, 0);
    let experienced = get_participant_ratings(players, contest_standings, 5);
    let top100 = top_k(&everyone, 100);

    let mut metrics_wt_sum = vec![
        pairwise_metric(&everyone),
        pairwise_metric(&experienced),
        pairwise_metric(top100),
        percentile_distance_metric(&everyone),
        percentile_distance_metric(&experienced),
        percentile_distance_metric(top100),
    ];
    for scale in (200..=600).step_by(50) {
        // In post-processing, only the best of these values should be kept, along with its scale
        metrics_wt_sum.push(cross_entropy_metric(&experienced, scale as f64));
    }

    PerformanceReport { metrics_wt_sum }
}
