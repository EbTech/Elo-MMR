extern crate multi_skill;

use multi_skill::data_processing::{get_dataset_by_name, ContestSummary};
use multi_skill::systems::{get_rating_system_by_name, simulate_contest, Player};

use std::cell::RefCell;
use std::collections::HashMap;

/*
    The following file alters the contest history of "tourist",
    the #1 competitive programmer of the past decade. The alterations
    consists of two phases:
        1) A volatility farming phase, where tourist alternatingly wins and loses
           (We do this by either retaining his performance in contest, or moving
            him to the bottom of the contest)
        2) A rating farming phase, where tourist performs as usual (near the top)
           and rapidly gains rating from his high volatility.
*/

/// Simulates the entire history of Codeforces
fn main() {
    // Parse the method we're applying the adversarial strategy on
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} method_name", args[0]);
        return;
    }

    let dataset = get_dataset_by_name("codeforces").unwrap();
    let seq_types = vec!["adversarial", "normal"];

    let (mu_noob, sig_noob) = (1500.0, 350.0);
    let max_contests = 400;

    for seq_type in seq_types {
        let system = get_rating_system_by_name(&args[1]).unwrap();
        let mut players = HashMap::<String, RefCell<Player>>::new();
        let mut summaries = Vec::with_capacity(dataset.len().min(max_contests));

        // The var below tracks tourist's score alterations.
        // First we'll have him advance to his nominal rating using the
        // first few contests (controlled by `initial_phase`).

        // We'll have him lose contests then win contests to bounce around
        // near his max rating

        // Then for the last portion of the contests (controlled by `win_phase`)
        // we'll have him perform as usual.
        let initial_phase = max_contests / 4;
        let win_time = 3 * max_contests / 4;
        let is_adversarial = seq_type == "adversarial";

        for (index, mut contest) in dataset.iter().enumerate().take(max_contests) {
            println!(
                "Processing{:6} contestants in{:5}th contest with wt={}: {}",
                contest.standings.len(),
                index,
                contest.weight,
                contest.name
            );

            if is_adversarial && index < win_time && index > initial_phase {
                if contest.has_contestant("tourist") {
                    let player = players["tourist"].borrow();
                    if player.approx_posterior.mu > 2600.0 {
                        contest.remove_contestant("tourist");
                        contest.push_contestant("tourist");
                    }
                }
            }

            simulate_contest(&mut players, &contest, &*system, mu_noob, sig_noob, index);
            summaries.push(ContestSummary::new(&contest));
        }

        let dir = std::path::PathBuf::from("../data/output");
        std::fs::create_dir_all(&dir.join("adversarial")).expect("Could not create directory");

        // Print contest histories of top players to data/output/players/{handle}.json
        let player = players["tourist"].borrow();

        let player_file = dir.join(format!(
            "adversarial/tourist_{}_{}.json",
            &args[1], seq_type
        ));
        let player_json =
            serde_json::to_string_pretty(&player.event_history).expect("Serialization error");
        std::fs::write(&player_file, player_json).expect("Failed to write to cache");
        println!("Wrote to {:?}", player_file);
    }
}
