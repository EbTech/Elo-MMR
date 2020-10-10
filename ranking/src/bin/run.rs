extern crate ranking;

use ranking::compute_ratings::simulate_contest;
use ranking::contest_config::{get_contest, get_contest_config, get_contest_ids, ContestSource};
use ranking::summary::print_ratings;
use std::collections::HashMap;

/// simulates the entire history of Codeforces; runs on my laptop in 90 mins,
/// somewhat longer if the Codeforces API data isn't cached
fn main() {
    let mut players = HashMap::new();
    let config = get_contest_config(ContestSource::Codeforces);
    let mut system = ranking::EloRSystem::default();
    let mut last_contest_time = 0;
    for contest_id in get_contest_ids(&config.contest_id_file) {
        let contest = get_contest(&config.contest_cache_folder, contest_id);
        println!(
            "Processing {:5} contestants in contest/{:4}: {}",
            contest.standings.len(),
            contest.id,
            contest.name
        );
        simulate_contest(&mut players, &contest, &mut system, 1500., 350.);
        last_contest_time = contest.time_seconds;
    }
    print_ratings(&players, last_contest_time - 183 * 86_400);
}
