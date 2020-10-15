use crate::compute_ratings::RatingSystem;
use crate::contest_config::ContestSource;

#[allow(unused_imports)]
use crate::{CodeforcesSystem, EloRSystem, TopCoderSystem, TrueSkillSPBSystem};

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
    pub contest_source: ContestSource,
}

pub fn load_experiment(source: impl AsRef<Path>) -> Experiment {
    let params_json = std::fs::read_to_string(source).expect("Failed to read parameters file");
    let params: ExperimentConfig =
        serde_json::from_str(&params_json).expect("Failed to parse parameters as JSON");

    println!("Loading rating system:\n{:#?}", params);
    let source = match params.contest_source.as_str() {
        "codeforces" => ContestSource::Codeforces,
        "reddit" => ContestSource::Reddit,
        "stackoverflow" => ContestSource::StackOverflow,
        "topcoder" => ContestSource::TopCoder,
        "synthetic" => ContestSource::Synthetic,
        _ => ContestSource::NotFound,
    };

    let rating_system: Box<dyn RatingSystem> = match params.system.method.as_str() {
        "codeforces" => Box::new(CodeforcesSystem {
            sig_perf: params.system.params[0],
            weight: params.system.params[1],
        }),
        "topcoder" => Box::new(TopCoderSystem {
            weight_multiplier: params.system.params[0],
        }),
        "elor-x" => Box::new(EloRSystem {
            sig_perf: params.system.params[0],
            sig_drift: params.system.params[1],
            split_ties: if params.system.params[2] > 0. { true } else { false },
            variant: crate::EloRVariant::Gaussian,
        }),
        "elor" => Box::new(EloRSystem {
            sig_perf: params.system.params[0],
            sig_drift: params.system.params[1],
            split_ties: if params.system.params[2] > 0. { true } else { false },
            variant: crate::EloRVariant::Logistic(params.system.params[3]),
        }),
        "trueskill" => Box::new(TrueSkillSPBSystem {
            eps: params.system.params[0],
            beta: params.system.params[1],
            convergence_eps: params.system.params[2],
            sigma_growth: params.system.params[3],
        }),
        x => panic!("'{}' is not a valid system name!", x),
    };

    Experiment {
        max_contests: params.max_contests,
        mu_noob: params.mu_noob,
        sig_noob: params.sig_noob,
        system: rating_system,
        contest_source: source,
    }
}