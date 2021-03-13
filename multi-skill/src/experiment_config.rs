use crate::data_processing::{get_dataset_by_name, subrange, Contest, Dataset};
use crate::systems::{
    simulate_contest, CodeforcesSys, EloMMR, EloMMRVariant, Glicko, PlayersByName, RatingSystem,
    TopcoderSys, TrueSkillSPb, BAR,
};

use crate::metrics::{compute_metrics_custom, PerformanceReport};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct SystemParams {
    pub method: String,
    pub params: Vec<f64>,
}

fn usize_max() -> usize {
    usize::MAX
}

#[allow(dead_code)]
fn is_usize_max(&num: &usize) -> bool {
    num == usize_max()
}

#[derive(Deserialize, Debug)]
pub struct ExperimentConfig {
    #[serde(default = "usize_max", skip_serializing_if = "is_usize_max")]
    pub max_contests: usize,
    pub mu_noob: f64,
    pub sig_noob: f64,
    pub system: SystemParams,
    pub contest_source: String,
}

pub struct Experiment {
    pub mu_noob: f64,
    pub sig_noob: f64,

    // Experiment should implement Send so that it can be sent across threads
    pub system: Box<dyn RatingSystem + Send>,
    pub dataset: Box<dyn Dataset<Item = Contest> + Send>,
}

impl Experiment {
    pub fn from_file(source: impl AsRef<Path>) -> Self {
        // Use json5 instead of serde_json to correctly parse f64::INFINITY
        let params_json = std::fs::read_to_string(source).expect("Failed to read parameters file");
        let params = json5::from_str(&params_json).expect("Failed to parse params as JSON");
        Self::from_config(params)
    }

    pub fn from_config(params: ExperimentConfig) -> Self {
        tracing::info!("Loading rating system:\n{:?}", params);
        let dataset = get_dataset_by_name(&params.contest_source).unwrap();
        let dataset_len = dataset.len().min(params.max_contests);
        let dataset = Box::new(subrange(dataset, ..dataset_len));

        let system: Box<dyn RatingSystem + Send> = match params.system.method.as_str() {
            "glicko" => Box::new(Glicko {
                beta: params.system.params[0],
                sig_drift: params.system.params[1],
            }),
            "bar" => Box::new(BAR {
                beta: params.system.params[0],
                sig_drift: params.system.params[1],
                kappa: 1e-4,
            }),
            "codeforces" => Box::new(CodeforcesSys {
                beta: params.system.params[0],
                weight_multiplier: params.system.params[1],
            }),
            "topcoder" => Box::new(TopcoderSys {
                weight_multiplier: params.system.params[0],
            }),
            "trueskill" => Box::new(TrueSkillSPb {
                eps: params.system.params[0],
                beta: params.system.params[1],
                convergence_eps: params.system.params[2],
                sig_drift: params.system.params[3],
            }),
            "mmx" => Box::new(EloMMR {
                beta: params.system.params[0],
                sig_limit: params.system.params[1],
                drift_per_sec: 0.,
                split_ties: params.system.params[2] > 0.,
                subsample_size: params.system.params[3] as usize,
                subsample_bucket: params.system.params[4],
                variant: EloMMRVariant::Gaussian,
            }),
            "mmr" => Box::new(EloMMR {
                beta: params.system.params[0],
                sig_limit: params.system.params[1],
                drift_per_sec: 0.,
                split_ties: params.system.params[2] > 0.,
                subsample_size: params.system.params[3] as usize,
                subsample_bucket: params.system.params[4],
                variant: EloMMRVariant::Logistic(params.system.params[5]),
            }),
            x => panic!("'{}' is not a valid system name!", x),
        };

        Self {
            mu_noob: params.mu_noob,
            sig_noob: params.sig_noob,
            system,
            dataset,
        }
    }

    pub fn eval(&self, num_rounds_postpone_eval: usize) -> ExperimentResults {
        let mut players = HashMap::new();
        let mut avg_perf = compute_metrics_custom(&mut players, &[]);

        // Run the contest histories and measure
        let now = std::time::Instant::now();
        for (index, contest) in self.dataset.iter().enumerate() {
            // Evaluate the non-training set; predictions should not use the contest
            // that they're predicting, so this step precedes simulation
            if index >= num_rounds_postpone_eval {
                avg_perf += compute_metrics_custom(&mut players, &contest.standings);
            }

            tracing::debug!(
                "Processing\n{:6} contestants in{:5}th contest with wt={}: {}",
                contest.standings.len(),
                index,
                contest.weight,
                contest.name
            );

            // Now run the actual rating update
            simulate_contest(
                &mut players,
                &contest,
                &*self.system,
                self.mu_noob,
                self.sig_noob,
                index,
            );
        }
        let secs_elapsed = now.elapsed().as_nanos() as f64 * 1e-9;

        ExperimentResults {
            players,
            avg_perf,
            secs_elapsed,
        }
    }

    pub fn eval_split(
        &self,
        num_rounds_postpone_eval: usize,
        max_participants: usize,
        rng_seed: u64,
    ) -> ExperimentResults {
        let mut players = HashMap::new();
        let mut avg_perf = compute_metrics_custom(&mut players, &[]);

        let mut rng = StdRng::seed_from_u64(rng_seed);
        let now = std::time::Instant::now();
        // Alternatively: .iter().flat_map(|contest| contest.random_split(n, &mut rng))
        // The reason we don't do this is that we want the original train-test split's index.
        for (index, contest) in self.dataset.iter().enumerate() {
            let split_contests = contest.random_split(max_participants, &mut rng);
            tracing::debug!(
                "Split{:5}th contest into{:6} subcontests",
                index,
                split_contests.size_hint().0
            );

            for subcontest in split_contests {
                // Evaluate the non-training set; predictions should not use the contest
                // that they're predicting, so this step precedes simulation
                if index >= num_rounds_postpone_eval {
                    avg_perf += compute_metrics_custom(&mut players, &subcontest.standings);
                }

                // Now run the actual rating update
                simulate_contest(
                    &mut players,
                    &subcontest,
                    &*self.system,
                    self.mu_noob,
                    self.sig_noob,
                    index,
                );
            }
        }
        let secs_elapsed = now.elapsed().as_nanos() as f64 * 1e-9;

        ExperimentResults {
            players,
            avg_perf,
            secs_elapsed,
        }
    }
}

pub struct ExperimentResults {
    pub players: PlayersByName,
    pub avg_perf: PerformanceReport,
    pub secs_elapsed: f64,
}
