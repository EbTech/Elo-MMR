use crate::data_processing::{get_dataset_by_name, ContestDataset, Dataset};
use crate::systems::{
    simulate_contest, CodeforcesSys, EloMMR, EloMMRVariant, EndureElo, Glicko, PlayersByName,
    RatingSystem, SimpleEloMMR, TopcoderSys, TrueSkillSPb, BAR,
};

use crate::data_processing::{read_json, write_json};
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

fn usize_zero() -> usize {
    0
}

fn usize_max() -> usize {
    usize::MAX
}

#[allow(dead_code)]
fn is_usize_zero(&num: &usize) -> bool {
    num == usize_zero()
}

#[allow(dead_code)]
fn is_usize_max(&num: &usize) -> bool {
    num == usize_max()
}

#[derive(Deserialize, Debug)]
pub struct ExperimentConfig {
    #[serde(default = "usize_zero", skip_serializing_if = "is_usize_zero")]
    pub skip_contests: usize,
    #[serde(default = "usize_max", skip_serializing_if = "is_usize_max")]
    pub max_contests: usize,
    pub mu_noob: f64,
    pub sig_noob: f64,
    pub system: SystemParams,
    pub contest_source: String,
    pub load_checkpoint: Option<String>,
    pub save_checkpoint: Option<String>,
}

impl ExperimentConfig {
    pub fn from_file(source: impl AsRef<Path>) -> Self {
        // Use json5 instead of serde_json to correctly parse f64::INFINITY
        let params_json = std::fs::read_to_string(source).expect("Failed to read parameters file");
        json5::from_str(&params_json).expect("Failed to parse params as JSON")
    }
}

pub struct Experiment {
    pub mu_noob: f64,
    pub sig_noob: f64,
    // Experiment should implement Send so that it can be sent across threads
    pub system: Box<dyn RatingSystem + Send>,
    pub dataset: ContestDataset,
    pub loaded_state: PlayersByName,
    pub save_checkpoint: Option<String>,
}

impl Experiment {
    pub fn from_config(config: ExperimentConfig) -> Self {
        tracing::info!("Loading rating system:\n{:?}", config);
        let dataset_full = get_dataset_by_name(&config.contest_source).unwrap();
        let dataset_end = dataset_full
            .len()
            .min(config.skip_contests + config.max_contests);
        let dataset = dataset_full
            .subrange(config.skip_contests..dataset_end)
            .boxed();

        let system: Box<dyn RatingSystem + Send> = match config.system.method.as_str() {
            "glicko" => Box::new(Glicko {
                beta: config.system.params[0],
                sig_drift: config.system.params[1],
            }),
            "bar" => Box::new(BAR {
                beta: config.system.params[0],
                sig_drift: config.system.params[1],
                kappa: 1e-4,
            }),
            "endure" => Box::new(EndureElo {
                beta: config.system.params[0],
                sig_drift: config.system.params[1],
            }),
            "cfsys" => Box::new(CodeforcesSys {
                beta: config.system.params[0],
                weight: config.system.params[1],
            }),
            "tcsys" => Box::new(TopcoderSys {
                weight_noob: config.system.params[0],
                weight_limit: config.system.params[1],
            }),
            "trueskill" => Box::new(TrueSkillSPb {
                eps: config.system.params[0],
                beta: config.system.params[1],
                convergence_eps: config.system.params[2],
                sig_drift: config.system.params[3],
            }),
            "mmx" => Box::new(EloMMR {
                weight_limit: config.system.params[0],
                noob_delay: vec![], // TODO: add this to the config spec
                sig_limit: config.system.params[1],
                drift_per_sec: 0.,
                split_ties: config.system.params[2] > 0.,
                subsample_size: config.system.params[3] as usize,
                subsample_bucket: config.system.params[4],
                variant: EloMMRVariant::Gaussian,
            }),
            "mmr" => Box::new(EloMMR {
                weight_limit: config.system.params[0],
                noob_delay: vec![], // TODO: add this to the config spec
                sig_limit: config.system.params[1],
                drift_per_sec: 0.,
                split_ties: config.system.params[2] > 0.,
                subsample_size: config.system.params[3] as usize,
                subsample_bucket: config.system.params[4],
                variant: EloMMRVariant::Logistic(config.system.params[5]),
            }),
            "mmr-simple" => Box::new(SimpleEloMMR {
                weight_limit: config.system.params[0],
                noob_delay: vec![0.6, 0.8], // TODO: add this to the config spec
                sig_limit: config.system.params[1],
                drift_per_sec: 0.,
                split_ties: config.system.params[2] > 0.,
                history_len: config.system.params[3] as usize,
                transfer_speed: config.system.params[4],
            }),
            x => panic!("'{}' is not a valid system name!", x),
        };

        let loaded_state = match config.load_checkpoint {
            Some(filename) => read_json(filename).expect("Failed to read checkpoint"),
            None => HashMap::new(),
        };

        Self {
            mu_noob: config.mu_noob,
            sig_noob: config.sig_noob,
            system,
            dataset,
            loaded_state,
            save_checkpoint: config.save_checkpoint,
        }
    }

    pub fn eval(&self, num_rounds_postpone_eval: usize) -> ExperimentResults {
        let mut players = self.loaded_state.clone();
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
                contest.rating_params.weight,
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

        if let Some(filename) = &self.save_checkpoint {
            write_json(&players, filename).expect("Failed to save checkpoint");
        }

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
        let mut players = self.loaded_state.clone();
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

        if let Some(filename) = &self.save_checkpoint {
            write_json(&players, filename).expect("Failed to save checkpoint");
        }

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
