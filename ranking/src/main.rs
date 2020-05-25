mod compute_ratings;
mod read_codeforces;

use compute_ratings::{print_ratings, simulate_contest};
use read_codeforces::{get_contest_ids, get_contest};
use std::collections::HashMap;

/// simulates the entire history of Codeforces, runs on my laptop in 25 minutes
fn main() {
    let mut players = HashMap::new();
    for contest_id in get_contest_ids() {
        let contest = get_contest(contest_id);
        println!(
            "Processing {:5} contestants in contest/{:4}: {}",
            contest.standings.len(),
            contest.id,
            contest.name
        );
        simulate_contest(&mut players, &contest);
    }
    print_ratings(&players, &get_contest_ids());
}
