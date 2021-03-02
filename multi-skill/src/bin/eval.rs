use rayon::prelude::*;

use multi_skill::experiment_config::Experiment;

fn main() {
    tracing_subscriber::fmt::init();

    // Load system configs from parameter files
    let mut experiment_files = vec![];
    let datasets = vec!["codeforces", "topcoder", "reddit", "synth-sm", "synth-la"];
    let methods = vec!["glicko", "bar", "cf", "tc", "ts", "mmx-fast", "mmr-fast"];
    let metrics = vec!["acc", "rnk", "ent"];

    for dataset in &datasets {
        for method in &methods {
            for metric in &metrics {
                let filename = format!("../experiments/{}/{}-{}.json", dataset, method, metric);
                experiment_files.push(filename);
            }
        }
    }

    // An override to do eval on a single experiment
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 2 && args[1] == "file:" {
        experiment_files = args[2..].to_vec();
    }

    experiment_files.par_iter().for_each(|filename| {
        let experiment = Experiment::from_file(filename);
        let train_set_len = experiment.dataset.len() / 10;
        let results = experiment.eval(train_set_len);

        let horizontal = "============================================================";
        tracing::info!(
            "{} {:?}: {}, {}s\n{}",
            filename,
            experiment.system,
            results.avg_perf,
            results.secs_elapsed,
            horizontal
        );
    });
}
