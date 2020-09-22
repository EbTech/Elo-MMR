mod compute_ratings;
mod contest_config;
mod read_codeforces;

mod cf_system;
mod tc_system;
mod elor_system;
mod ts_system;

use compute_ratings::{print_ratings, simulate_contest};
use contest_config::{get_contest, get_contest_config, get_contest_ids, ContestSource};
use std::collections::HashMap;

/// simulates the entire history of Codeforces; runs on my laptop in an hour,
/// somewhat longer if the Codeforces API data isn't cached
fn main() {
    let mut players = HashMap::new();
    let config = get_contest_config(ContestSource::Codeforces);
    let mut system = ts_system::TrueSkillSPBSystem::default();
    let mut last_contest_time = 0;
    for contest_id in get_contest_ids(&config.contest_id_file) {
        let contest = get_contest(&config.contest_cache_folder, contest_id);
        println!(
            "Processing {:5} contestants in contest/{:4}: {}",
            contest.standings.len(),
            contest.id,
            contest.name
        );
        simulate_contest(&mut players, &contest, &mut system);
        last_contest_time = contest.time_seconds;
    }
    print_ratings(&players, last_contest_time - 183 * 86_400);
}
