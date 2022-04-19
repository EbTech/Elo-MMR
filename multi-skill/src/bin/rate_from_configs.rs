use multi_skill::data_processing::Dataset;
use multi_skill::experiment_config::{Experiment, ExperimentConfig};

fn main() {
    tracing_subscriber::fmt::init();

    // Parse arguments, prepare rating system and datasets
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        tracing::error!("Usage: {} <list of config files>", args[0]);
        return;
    }

    // Run each config file in turn. Note that there is no code here to display or save
    // results, so this binary is best used with configs that save to checkpoint files.
    for ex in args[1..]
        .iter()
        .map(ExperimentConfig::from_file)
        .map(Experiment::from_config)
    {
        // Simulate the contests and rating updates
        let dataset_len = ex.dataset.len();
        let results = ex.eval(dataset_len);
        tracing::info!(
            "This experiment finished in {} seconds.",
            results.secs_elapsed,
        );
    }
}
