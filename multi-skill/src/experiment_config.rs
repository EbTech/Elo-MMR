use crate::data_processing::{get_dataset_by_name, Contest, Dataset};
use crate::systems::{
    simulate_contest, CodeforcesSys, EloMMR, EloMMRVariant, Glicko, RatingSystem, TopcoderSys,
    TrueSkillSPb, BAR,
};

use crate::metrics::compute_metrics_custom;
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
    pub max_contests: usize,
    pub mu_noob: f64,
    pub sig_noob: f64,

    // Experiment should implement Send so that it can be sent across threads
    pub system: Box<dyn RatingSystem + Send>,
    pub dataset: Box<dyn Dataset<Item = Contest> + Send>,
}

impl Experiment {
    pub fn from_file(source: impl AsRef<Path>) -> Self {
        let params_json = std::fs::read_to_string(source).expect("Failed to read parameters file");
        let params = serde_json::from_str(&params_json).expect("Failed to parse params as JSON");
        Self::from_config(params)
    }

    pub fn from_config(params: ExperimentConfig) -> Self {
        println!("Loading rating system:\n{:#?}", params);
        let dataset = get_dataset_by_name(&params.contest_source).unwrap();

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
                variant: EloMMRVariant::Gaussian,
            }),
            "mmr" => Box::new(EloMMR {
                beta: params.system.params[0],
                sig_limit: params.system.params[1],
                drift_per_sec: 0.,
                split_ties: params.system.params[2] > 0.,
                variant: EloMMRVariant::Logistic(params.system.params[3]),
            }),
            x => panic!("'{}' is not a valid system name!", x),
        };

        Self {
            max_contests: params.max_contests,
            mu_noob: params.mu_noob,
            sig_noob: params.sig_noob,
            system,
            dataset,
        }
    }

    pub fn eval(self, mut num_rounds_postpone_eval: usize, tag: &str) {
        let mut players = HashMap::new();
        let mut avg_perf = compute_metrics_custom(&mut players, &[]);

        // Run the contest histories and measure
        let now = std::time::Instant::now();
        for contest in self.dataset.iter().take(self.max_contests) {
            // Evaludate the non-training set; predictions should not use the contest
            // that they're predicting, so this step precedes simulation
            if num_rounds_postpone_eval > 0 {
                num_rounds_postpone_eval -= 1;
            } else {
                avg_perf += compute_metrics_custom(&mut players, &contest.standings);
            }

            // Now run the actual rating update
            simulate_contest(
                &mut players,
                &contest,
                &*self.system,
                self.mu_noob,
                self.sig_noob,
            );
        }
        let secs_elapsed = now.elapsed().as_nanos() as f64 * 1e-9;

        let horizontal = "============================================================";
        let output = format!(
            "{} {:?}: {}, {}s\n{}",
            tag, self.system, avg_perf, secs_elapsed, horizontal
        );
        println!("{}", output);
    }
}
