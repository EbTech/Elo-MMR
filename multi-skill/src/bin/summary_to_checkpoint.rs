use multi_skill::data_processing::{read_csv, write_json};
use multi_skill::systems::{Player, PlayersByName};
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
        tracing::error!("Usage: {} dataset_name", args[0]);
        return;
    }
    let contest_source = &args[1];
    let dir = std::path::PathBuf::from("../data").join(&contest_source);
    let input_file = dir.join("all_players.csv");
    let output_file = dir.join("checkpoint_players.json");

    let summary = read_csv(input_file, true).expect("Failed to read summaries");
    let checkpoint = make_checkpoint(summary);
    write_json(&checkpoint, output_file).expect("Failed to write checkpoint");
}
