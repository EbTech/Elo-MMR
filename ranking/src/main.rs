mod compute_ratings;
mod contest_config;
mod read_codeforces;

use compute_ratings::{print_ratings, simulate_contest};
use contest_config::{get_contest, get_contest_config, get_contest_ids, ContestSource};
use std::collections::HashMap;

/// simulates the entire history of Codeforces; runs on my laptop in 28 minutes,
/// somewhat longer if accessing the Codeforces API
fn main() {
    let mut players = HashMap::new();
    let config = get_contest_config(ContestSource::Reddit);
    for contest_id in get_contest_ids(&config.contest_id_file) {
        let contest = get_contest(&config.contest_cache_folder, contest_id);
        println!(
            "Processing {:5} contestants in contest/{:4}: {}",
            contest.standings.len(),
            contest.id,
            contest.name
        );
        simulate_contest(&mut players, &contest);
    }
    print_ratings(&players, &get_contest_ids(&config.contest_id_file));
}
