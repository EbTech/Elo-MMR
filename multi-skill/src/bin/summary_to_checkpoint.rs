use multi_skill::data_processing::{read_csv, write_json};
use multi_skill::systems::{Player, PlayerEvent, PlayersByName};
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
                display_rating: 0,
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
        tracing::error!("Usage: {} dataset_name", args[0]);
        return;
    }
    let contest_source = &args[1];
    let dir = std::path::PathBuf::from("../data").join(contest_source);
    let input_file = dir.join("all_players.csv");
    let output_file = dir.join("checkpoint_players.json");

    let summary = read_csv(input_file, true).expect("Failed to read summaries");
    let checkpoint = make_checkpoint(summary);
    write_json(&checkpoint, output_file).expect("Failed to write checkpoint");
}
