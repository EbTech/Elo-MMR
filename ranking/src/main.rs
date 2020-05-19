mod compute_ratings;

use compute_ratings::{get_contests, print_ratings, simulate_contest};
use std::collections::HashMap;

fn main() {
    // simulates the entire history of Codeforces, runs on my laptop in 15 minutes
    let mut players = HashMap::new();
    for contest in get_contests() {
        simulate_contest(&mut players, contest);
    }
    print_ratings(&players);
}
