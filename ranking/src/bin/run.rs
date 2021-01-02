extern crate ranking;

use ranking::compute_ratings::{simulate_contest, RatingSystem};
use ranking::contest_config::get_dataset_by_name;
use ranking::summary::print_ratings;
use std::collections::HashMap;

fn get_rating_system_by_name(system_name: &str) -> Result<Box<dyn RatingSystem>, String> {
    match system_name {
        "glicko" => Ok(Box::new(ranking::GlickoSystem::default())),
        "cf" => Ok(Box::new(ranking::CodeforcesSystem::default())),
        "tc" => Ok(Box::new(ranking::TopCoderSystem::default())),
        "ts" => Ok(Box::new(ranking::TrueSkillSPBSystem::default())),
        "mmx" => Ok(Box::new(ranking::EloRSystem::default_gaussian())),
        "mmr" => Ok(Box::new(ranking::EloRSystem::default())),
        name => Err(format!(
            "{} is not a valid rating system. Must be one of: glicko, cf, tc, ts, mmx, mmr",
            name
        )),
    }
}

/// Simulates the entire history of Codeforces
fn main() {
    // Parse arguments, prepare rating system and datasets
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 && args.len() != 4 {
        eprintln!("Usage: {} system_name dataset_name [num_contests]", args[0]);
        return;
    }
    let system = get_rating_system_by_name(&args[1]).unwrap();
    let dataset = get_dataset_by_name(&args[2]).unwrap();
    let max_contests = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);

    // Simulate the contests and rating updates
    let mut players = HashMap::new();
    let mut last_contest_time = 0;
    for contest in dataset.iter().take(max_contests) {
        println!(
            "Processing {:5} contestants in contest/{:4}: {}",
            contest.standings.len(),
            contest.id,
            contest.name
        );
        simulate_contest(&mut players, &contest, &*system, 1500., 300.);
        last_contest_time = contest.time_seconds;
    }
    print_ratings(&players, last_contest_time.saturating_sub(183 * 86_400));
}
