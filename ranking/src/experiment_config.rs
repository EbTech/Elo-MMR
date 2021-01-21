use crate::data_processing::{get_dataset_by_name, Contest, Dataset};
use crate::systems::{
    CodeforcesSys, EloMMR, EloMMRVariant, Glicko, RatingSystem, TopcoderSys, TrueSkillSPb, BAR,
};

use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct SystemParams {
    pub method: String,
    pub params: Vec<f64>,
}

#[derive(Deserialize, Debug)]
pub struct ExperimentConfig {
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
    pub system: Box<dyn RatingSystem>,
    pub dataset: Box<dyn Dataset<Item = Contest>>,
}

pub fn load_experiment(source: impl AsRef<Path>) -> Experiment {
    let params_json = std::fs::read_to_string(source).expect("Failed to read parameters file");
    let params: ExperimentConfig =
        serde_json::from_str(&params_json).expect("Failed to parse parameters as JSON");

    println!("Loading rating system:\n{:#?}", params);
    let dataset = get_dataset_by_name(&params.contest_source).unwrap();

    let system: Box<dyn RatingSystem> = match params.system.method.as_str() {
        "bar" => Box::new(BAR {
            sig_perf: params.system.params[0],
            sig_drift: params.system.params[1],
            kappa: 1e-4,
        }),
        "glicko" => Box::new(Glicko {
            sig_perf: params.system.params[0],
            sig_drift: params.system.params[1],
        }),
        "codeforces" => Box::new(CodeforcesSys {
            sig_perf: params.system.params[0],
            weight: params.system.params[1],
        }),
        "topcoder" => Box::new(TopcoderSys {
            weight_multiplier: params.system.params[0],
        }),
        "trueskill" => Box::new(TrueSkillSPb {
            eps: params.system.params[0],
            beta: params.system.params[1],
            convergence_eps: params.system.params[2],
            sigma_growth: params.system.params[3],
        }),
        "elor-x" => Box::new(EloMMR {
            sig_perf: params.system.params[0],
            sig_drift: params.system.params[1],
            split_ties: params.system.params[2] > 0.,
            variant: EloMMRVariant::Gaussian,
        }),
        "elor" => Box::new(EloMMR {
            sig_perf: params.system.params[0],
            sig_drift: params.system.params[1],
            split_ties: params.system.params[2] > 0.,
            variant: EloMMRVariant::Logistic(params.system.params[3]),
        }),
        x => panic!("'{}' is not a valid system name!", x),
    };

    Experiment {
        max_contests: params.max_contests,
        mu_noob: params.mu_noob,
        sig_noob: params.sig_noob,
        system,
        dataset,
    }
}
