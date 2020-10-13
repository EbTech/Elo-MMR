use crate::contest_config::ContestSource;
use crate::compute_ratings::RatingSystem;

#[allow(unused_imports)]
use crate::{CodeforcesSystem, EloRSystem, TopCoderSystem, TrueSkillSPBSystem};

use serde::{Deserialize};
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct SystemParams {
    pub method: String,
    pub params: Vec<f64>,
}

#[derive(Deserialize, Debug)]
pub struct ExperimentFile {
    pub max_contests: usize,
    pub mu_noob: f64,
    pub sig_noob: f64,
    pub topk: usize,
    pub system: SystemParams,
    pub contest_source: String,
}

pub struct Experiment {
    pub max_contests: usize,
    pub mu_noob: f64,
    pub sig_noob: f64,
    pub topk: usize,
    pub system: Box<dyn RatingSystem>,
    pub contest_source: ContestSource,
}

pub fn load_experiment<P: AsRef<Path>>(source: &P) -> Experiment {
    let params_json =
        std::fs::read_to_string(source).expect("Failed to read parameters file");
    let params: ExperimentFile = serde_json::from_str(&params_json).expect("Failed to parse parameters as JSON");

    println!("Loading rating system:\n{:#?}", params);
    let source = match &params.contest_source[..] {
        "codeforces" => ContestSource::Codeforces,
        "reddit" => ContestSource::Reddit,
        "stackoverflow" => ContestSource::StackOverflow,
        "synthetic" => ContestSource::Synthetic,
        _ => ContestSource::NotFound,
    };

    let system: Box<dyn RatingSystem> = match &params.system.method[..] {
        "codeforces" => Box::new(CodeforcesSystem { 
            sig_perf: params.system.params[0], 
            weight: params.system.params[1] }),
        "topcoder" => Box::new(TopCoderSystem {
            weight_multiplier: params.system.params[0] }),
        "elor" => Box::new(EloRSystem {
            sig_perf: params.system.params[0],
            sig_drift: params.system.params[1],
            variant: crate::elor_system::EloRVariant::Logistic(params.system.params[2]),
            split_ties: false }),
        "trueskill" => Box::new(TrueSkillSPBSystem {
            eps: params.system.params[0],
            beta: params.system.params[1],
            convergence_eps: params.system.params[2],
            sigma_growth: params.system.params[3] }),
        _ => Box::new(EloRSystem::default()),
    };

    // format!("../data/{}/contest_ids.json", source_name)
    Experiment {
        max_contests: params.max_contests,
        mu_noob: params.mu_noob,
        sig_noob: params.sig_noob,
        topk: params.topk,
        system: system,
        contest_source: source,
    }
}