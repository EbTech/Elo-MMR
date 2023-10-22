use multi_skill::data_processing::try_write_json;
use multi_skill::experiment_config::{Experiment, ExperimentConfig};
use multi_skill::systems::PlayerEvent;
use std::ops::DerefMut;

fn main() {
    tracing_subscriber::fmt::init();

    // Parse arguments, prepare rating system and datasets
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        tracing::error!("Usage: {} experiment_file handle perf_score", args[0]);
        return;
    }
    let experiment_file = &args[1];
    let handle = &args[2];
    let perf_score = args[3]
        .parse::<f64>()
        .expect("Third argument (perf_score) must be a number!");

    // Extract the requested player, imitating the steps in systems::common::simulate_contest()
    // TODO: remove the unnecessary file I/O that's slowing down this script
    let config = ExperimentConfig::from_file(experiment_file);
    let ex = Experiment::from_config(config);
    let mut player_guard = ex
        .loaded_state
        .get(handle)
        .expect("Handle was not found in the loaded state") // TODO: fallback to default?
        .borrow_mut();
    let player = player_guard.deref_mut();
    // TODO: should the following be refactored, both here and in simulate_contest()?
    // Note that contest_index, place, delta_time and update_time are not available
    // and therefore cannot be set correctly in this script.
    player.event_history.push(PlayerEvent {
        contest_index: 0, // this information is not available
        rating_mu: 0,     // will be filled by system.individual_update()
        rating_sig: 0,    // will be filled by system.individual_update()
        perf_score: 0,    // will be filled by system.individual_update()
        place: 0,         // this information is not available
    });

    // Perform the rating update with the configured parameters
    let params = ex
        .dataset
        .into_iter()
        .next()
        .expect("Configured dataset should not be empty")
        .rating_params;
    ex.system.individual_update(params, player, perf_score);

    // Output the result to data/<handle>.json
    let dir = std::path::PathBuf::from("../data");
    std::fs::create_dir_all(dir.join("players")).expect("Could not create directory");
    let output_file = dir.join(format!("{}.json", handle));
    try_write_json(player, output_file);
}
