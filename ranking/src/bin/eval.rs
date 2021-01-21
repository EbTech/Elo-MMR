extern crate ranking;
extern crate rayon;
use rayon::prelude::*;

use ranking::experiment_config::load_experiment;
use ranking::metrics::compute_metrics_custom;
use ranking::systems::simulate_contest;
use std::collections::HashMap;
use std::time::Instant;

fn main() {
    // Load system configs from parameter files
    let mut experiment_files = vec![];
    let datasets = vec!["codeforces", "topcoder", "reddit", "synthetic"];
    let methods = vec![/*glicko", "bar", */ "cf", "tc", "ts", "mmx", "mmr"];
    let metrics = vec!["acc", "rnk", "ent"];

    for dataset in &datasets {
        for method in &methods {
            for metric in &metrics {
                let filename = format!("../experiments/{}/{}-{}.json", dataset, method, metric);
                experiment_files.push(filename);
            }
        }
    }

    experiment_files.par_iter().for_each(|filename| {
        // Grab all the details of the experiment from a file
        let experiment = load_experiment(filename);
        let dataset = experiment.dataset;
        let system = experiment.system;
        let max_contests = experiment.max_contests;
        let mu_noob = experiment.mu_noob;
        let sig_noob = experiment.sig_noob;

        // In our experiments, max_contests should just be the entire dataset
        assert!(experiment.max_contests >= dataset.len());
        let num_rounds_to_fit = dataset.len() / 10;

        let mut players = HashMap::new();
        let mut avg_perf = compute_metrics_custom(&mut players, &[]);
        let now = Instant::now();

        // Run the contest histories and measure
        for (i, contest) in dataset.iter().enumerate().take(max_contests) {
            // Evaludate the non-training set; predictions should not use the contest
            // that they're predicting, so this step precedes simulation
            if i >= num_rounds_to_fit {
                avg_perf += compute_metrics_custom(&mut players, &contest.standings);
            }

            // Now run the actual rating update
            simulate_contest(&mut players, &contest, &*system, mu_noob, sig_noob);
        }
        println!(
            "{} {:?}: {}, {}s",
            filename,
            system,
            avg_perf,
            now.elapsed().as_nanos() as f64 * 1e-9
        );
        println!("=============================================================");
    });
}
