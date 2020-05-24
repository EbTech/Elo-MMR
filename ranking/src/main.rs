mod read_codeforces;
mod compute_ratings;

use read_codeforces::{get_contests, read_results};
use compute_ratings::{print_ratings, simulate_contest};
use std::collections::HashMap;

fn main() {
    // simulates the entire history of Codeforces, runs on my laptop in 24 minutes
    let contests = get_contests();
    let mut players = HashMap::new();
    for &contest in &contests {
        let (title, results) = read_results(contest);
        println!(
            "Processing {} contestants in contest/{}: {}",
            results.len(),
            contest,
            title
        );
        simulate_contest(&mut players, &results);
    }
    print_ratings(&players, &contests);
}
