mod compute_ratings;
mod read_codeforces;

use compute_ratings::{print_ratings, simulate_contest};
use read_codeforces::{get_contest_ids, read_results};
use std::collections::HashMap;

fn main() {
    // simulates the entire history of Codeforces, runs on my laptop in 24 minutes
    let contest_ids = get_contest_ids();
    let mut players = HashMap::new();
    for &contest_id in &contest_ids {
        let contest = read_results(contest_id);
        println!(
            "Processing {} contestants in contest/{}: {}",
            contest.standings.len(),
            contest.id,
            contest.name
        );
        simulate_contest(&mut players, &contest);
    }
    print_ratings(&players, &contest_ids);
}
