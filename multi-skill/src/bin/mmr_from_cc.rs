use multi_skill::data_processing::{get_dataset_by_name, read_csv, try_write_slice_to_file};
use multi_skill::summary::make_leaderboard;
use multi_skill::systems::{get_rating_system_by_name, simulate_contest, Player, PlayerEvent, PlayersByName};

use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SimplePlayer {
    handle: String,
    cur_mu: f64,
    cur_sigma: Option<f64>,
    num_contests: Option<usize>,
}

fn make_checkpoint(players: Vec<SimplePlayer>) -> PlayersByName {
    players
        .into_iter()
        .map(|simp| {
            // In priority order: use cur_sigma, num_contests, or a default
            let sig = match simp.cur_sigma {
                Some(sig) => sig,
                None => {
                    const SIG_LIM_SQ: f64 = 80. * 80.;
                    const WEIGHT: f64 = 0.2;
                    let sig_perf_sq = (1. + 1. / WEIGHT) * SIG_LIM_SQ;
                    let sig_drift_sq = WEIGHT * SIG_LIM_SQ;
                    let mut sig_sq = 350. * 350.;
                    for _ in 0..simp.num_contests.unwrap_or(1) {
                        sig_sq += sig_drift_sq;
                        sig_sq *= sig_perf_sq / (sig_sq + sig_perf_sq);
                    }
                    sig_sq.sqrt()
                }
            };

            // Hack to create a Player with a non-empty history,
            // when we don't have access to their actual history.
            let mut player = Player::with_rating(simp.cur_mu, sig, 0);
            let fake_event = PlayerEvent {
                contest_index: 0,
                rating_mu: 0,
                rating_sig: 0,
                perf_score: 0,
                place: 0,
            };
            player.event_history.push(fake_event);
            player.update_rating(player.approx_posterior, simp.cur_mu);
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

    // Set up the rating system
    let system = &args[1];
    let system = get_rating_system_by_name(system).unwrap();
    let dataset = get_dataset_by_name("codechef").unwrap();
    let mut mu_noob = 1500.;
    let sig_noob = 350.;
    let input_file =
        std::path::PathBuf::from("/home/work_space/elommr-data/cc_init_condition-MARCH15-146.csv");
    let summary = read_csv(input_file, true).expect("Failed to read summaries");
    let mut players = make_checkpoint(summary).clone();
    let contest_cutoff = 146;

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
    let dir =
        std::path::PathBuf::from("/home/work_space/elommr-data/elommr-checkpoints/start-from-146/");
    for (index, contest) in dataset.iter().enumerate() {
        if index <= contest_cutoff {
            continue;
        }

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
            let (_summary, rating_data) = make_leaderboard(&players, 0);
            try_write_slice_to_file(&rating_data, &output_file);
        }
    }
}
