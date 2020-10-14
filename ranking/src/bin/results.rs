extern crate ranking;

use ranking::compute_ratings::{get_participant_ratings, simulate_contest};
use ranking::contest_config::{get_contest, get_contest_config, get_contest_ids};
use ranking::experiment_config::load_experiment;
use ranking::metrics::{compute_metrics_custom, PerformanceReport};
use std::collections::HashMap;
use std::time::Instant;

#[allow(unused_imports)]
use ranking::{CodeforcesSystem, EloRSystem, TopCoderSystem, TrueSkillSPBSystem};

fn main() {
    // Load system configs from parameter files
    let experiment_files = vec![
        "../experiments/cf-experiment.json".to_string(),
        "../experiments/tc-experiment.json".into(),
        "../experiments/elor-experiment.json".into(),
        "../experiments/ts-experiment.json".into(),
    ];

    for filename in experiment_files.iter() {
        let experiment = load_experiment(&filename);

        let config = get_contest_config(experiment.contest_source);
        let contest_ids = get_contest_ids(&config.contest_id_file);

        let system = experiment.system;

        let max_contests = experiment.max_contests;
        let mu_noob = experiment.mu_noob;
        let sig_noob = experiment.sig_noob;
        let topk = experiment.topk;

        let mut players = HashMap::new();
        let mut avg_perf = PerformanceReport::new(3);
        let now = Instant::now();

        // Run the contest histories and measure
        for &contest_id in contest_ids.iter().take(max_contests) {
            let contest = get_contest(&config.contest_cache_folder, contest_id);

            // Predict performance must be run before simulate contest
            // since we don't want to make predictions after we've seen the contest
            let standings = get_participant_ratings(&mut players, &contest, 0);
            avg_perf += compute_metrics_custom(&standings, topk);

            // Now run the actual rating update
            simulate_contest(&mut players, &contest, &*system, mu_noob, sig_noob);
        }
        println!(
            "{:?}: {}, {}s",
            system,
            avg_perf,
            now.elapsed().as_millis() as f64 / 1000.
        );
        println!("=============================================================");
    }
}
