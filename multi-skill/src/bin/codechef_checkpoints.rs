use multi_skill::data_processing::{
    get_dataset_by_name, read_csv, try_write_slice_to_file, write_json,
};
use multi_skill::metrics::{compute_metrics_custom, PerformanceReport};
use multi_skill::summary::make_leaderboard;
use multi_skill::systems::{get_rating_system_by_name, simulate_contest, Player, PlayersByName};

use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SimplePlayer {
    handle: String,
    cur_mu: f64,
    cur_sigma: f64,
}

fn make_checkpoint(players: Vec<SimplePlayer>) -> PlayersByName {
    players
        .into_iter()
        .map(|simp| {
            let player = Player::with_rating(simp.cur_mu, simp.cur_sigma, 0);
            (simp.handle, RefCell::new(player))
        })
        .collect()
}

fn main() {
    tracing_subscriber::fmt::init();

    // Parse arguments, prepare rating system and datasets
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        tracing::error!("Usage: {} system_name", args[0]);
        return;
    }
    let system = &args[1];
    let system = get_rating_system_by_name(system).unwrap();
    let dataset = get_dataset_by_name("codechef").unwrap();
    let mut mu_noob = 1500.;
    let sig_noob = 350.;
    let mut players = std::collections::HashMap::new();
    let mut avg_perf = compute_metrics_custom(&mut players, &[]);

    // Get list of contest names to compare with Codechef's rating system
    let paths = std::fs::read_dir("/home/work_space/elommr-data/ratings").unwrap();
    let mut checkpoints = std::collections::HashSet::<String>::new();
    for path in paths {
        if let Some(contest_name) = path.unwrap().path().file_stem() {
            if let Some(string_name) = contest_name.to_os_string().into_string().ok() {
                checkpoints.insert(string_name);
            }
        }
    }

    // Run the contest histories and measure
    let dir = std::path::PathBuf::from("/home/work_space/elommr-data/elommr-checkpoints/codechef/");
    let now = std::time::Instant::now();
    for (index, contest) in dataset.iter().enumerate() {
        tracing::debug!(
            "Processing\n{:6} contestants in{:5}th contest with wt={}: {}",
            contest.standings.len(),
            index,
            contest.weight,
            contest.name
        );

        // At some point, codechef changed the default rating!
        if contest.name == "START25B" {
            mu_noob = 1000.;
        }

        // Now run the actual rating update
        simulate_contest(&mut players, &contest, &*system, mu_noob, sig_noob, index);

        if checkpoints.contains(&contest.name) {
            let output_file = dir.join(contest.name.clone() + ".csv");
            let (summary, rating_data) = make_leaderboard(&players, 0);
            try_write_slice_to_file(&rating_data, &output_file);
        }
    }
    let secs_elapsed = now.elapsed().as_nanos() as f64 * 1e-9;
}
