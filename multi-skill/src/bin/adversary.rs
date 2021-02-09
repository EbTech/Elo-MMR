extern crate multi_skill;

use multi_skill::data_processing::{get_dataset_by_name, write_slice_to_file};
use multi_skill::systems::{get_rating_system_by_name, simulate_contest, PlayersByName};

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
        eprintln!("Usage: {} method_name (suggestion: mmr)", args[0]);
        return;
    }

    let dataset = get_dataset_by_name("codeforces").unwrap();
    let seq_types = vec!["adversarial", "normal"];

    let (mu_noob, sig_noob) = (1500., 350.);
    let initial_phase = 128; // tourist's 45th
    let win_time = 346; // tourist's 90th (+45)
    let max_contests = 462; // tourist's 105th (+15)
    let tcoder_system = get_rating_system_by_name("tc").unwrap();
    let custom_system = get_rating_system_by_name(&args[1]).unwrap();

    for seq_type in seq_types {
        // The var below tracks tourist's score alterations.
        // First we'll have him advance to his nominal rating using the
        // first few contests (controlled by `initial_phase`).

        // We'll have him lose contests then win contests to bounce around
        // near his max rating

        // Then for the last portion of the contests (controlled by `win_phase`)
        // we'll have him perform as usual.
        let is_adversarial = seq_type == "adversarial";
        let mut tcoder_players = PlayersByName::new();
        let mut custom_players = PlayersByName::new();

        for (index, mut contest) in dataset.iter().enumerate().take(max_contests) {
            println!(
                "Processing{:6} contestants in{:5}th contest with wt={}: {}",
                contest.standings.len(),
                index,
                contest.weight,
                contest.name
            );

            if is_adversarial && initial_phase <= index && index < win_time {
                let player = tcoder_players["tourist"].borrow();
                if player.approx_posterior.mu > 2975.0 {
                    if contest.remove_contestant("tourist").is_some() {
                        contest.push_contestant("tourist");
                    }
                }
            }

            simulate_contest(
                &mut tcoder_players,
                &contest,
                &*tcoder_system,
                mu_noob,
                sig_noob,
                index,
            );
            simulate_contest(
                &mut custom_players,
                &contest,
                &*custom_system,
                mu_noob,
                sig_noob,
                index,
            );
        }

        let dir = std::path::PathBuf::from("../data/output/adversarial");
        std::fs::create_dir_all(&dir).expect("Could not create directory");

        // Print contest histories of top players to data/output/players/{handle}.json
        let player = tcoder_players["tourist"].borrow();
        let player_file = dir.join(format!("tourist_tc_{}.json", seq_type));
        write_slice_to_file(&player.event_history, &player_file);

        let player = custom_players["tourist"].borrow();
        let player_file = dir.join(format!("tourist_{}_{}.json", &args[1], seq_type));
        write_slice_to_file(&player.event_history, &player_file);
    }
}
