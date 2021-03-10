use multi_skill::data_processing::{get_dataset_by_name, subrange, write_slice_to_file};
use multi_skill::experiment_config::Experiment;
use multi_skill::summary::print_ratings;
use multi_skill::systems::get_rating_system_by_name;

fn get_experiment_from_args(args: &[String]) -> Experiment {
    let system = &args[1];
    let name = &args[2];

    if system == "file:" {
        Experiment::from_file(name)
    } else {
        let system = get_rating_system_by_name(system).unwrap();
        let mut dataset = get_dataset_by_name(name).unwrap();
        if let Some(num_contests) = args.get(3).and_then(|s| s.parse().ok()) {
            dataset = Box::new(subrange(dataset, 0..num_contests));
        }

        Experiment {
            mu_noob: 1500.,
            sig_noob: 350.,
            system,
            dataset,
        }
    }
}

/// Simulates the entire history of Codeforces
fn main() {
    tracing_subscriber::fmt::init();

    // Parse arguments, prepare rating system and datasets
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 && args.len() != 4 {
        tracing::error!("Usage: {} system_name dataset_name [num_contests]", args[0]);
        return;
    }
    let ex = get_experiment_from_args(&args);

    // Simulate the contests and rating updates
    let dataset_len = ex.dataset.len();
    let results = ex.eval(dataset_len);
    tracing::info!(
        "{:?}\nFinished in {} seconds.",
        ex.system,
        results.secs_elapsed,
    );

    let six_months_ago = ex
        .dataset
        .get(dataset_len - 1)
        .time_seconds
        .saturating_sub(183 * 86_400);
    let dir = std::path::PathBuf::from("../data/output");
    std::fs::create_dir_all(&dir.join("players")).expect("Could not create directory");

    // Print contest histories of top players to data/output/players/{handle}.csv
    for (handle, player) in &results.players {
        let player = player.borrow();

        //let last_event = player.event_history.last().expect("Empty history");
        //if last_event.rating_mu >= 2700 && player.update_time > six_months_ago {
        if true {
            let player_file = dir.join(format!("players/{}.csv", handle));
            write_slice_to_file(&player.event_history, &player_file);
        }
    }

    // Print ratings list to data/codeforces/CFratings.txt
    print_ratings(&results.players, six_months_ago, &dir);
}
