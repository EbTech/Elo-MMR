extern crate multi_skill;
extern crate rayon;
use rayon::prelude::*;

use multi_skill::experiment_config::Experiment;

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
        let experiment = Experiment::from_file(filename);

        // In our experiments, max_contests should just be the entire dataset
        assert!(experiment.max_contests >= experiment.dataset.len());
        let num_training_rounds = experiment.dataset.len() / 10;

        experiment.eval(num_training_rounds, filename);
    });
}
