extern crate ranking;

use ranking::compute_ratings::{simulate_contest, RatingSystem};
use ranking::contest_config::{get_contest, get_contest_config, get_contest_ids, ContestSource};
use ranking::summary::print_ratings;
use std::collections::HashMap;

/// simulates the entire history of Codeforces; runs on my laptop in 23 mins,
/// somewhat longer if the Codeforces API data isn't cached
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 && args.len() != 3 {
        eprintln!("Usage: supply one of the following arguments: glicko,cf,tc,ts,elor");
        return;
    }

    let system: Box<dyn RatingSystem> = match args[1].as_str() {
        "glicko" => Box::new(ranking::GlickoSystem::default()),
        "cf" => Box::new(ranking::CodeforcesSystem::default()),
        "tc" => Box::new(ranking::TopCoderSystem::default()),
        "ts" => Box::new(ranking::TrueSkillSPBSystem::default()),
        "elor" => Box::new(ranking::EloRSystem::default()),
        s => {
            eprintln!(
                "{} is not a valid rating system. Must be one of: glicko,cf,tc,ts,elor",
                s
            );
            return;
        }
    };
    let max_contests: usize = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);

    let mut players = HashMap::new();
    let config = get_contest_config(ContestSource::Codeforces);
    let mut last_contest_time = 0;
    for contest_id in get_contest_ids(&config.contest_id_file)
        .into_iter()
        .take(max_contests)
    {
        let contest = get_contest(&config.contest_cache_folder, contest_id);
        println!(
            "Processing {:5} contestants in contest/{:4}: {}",
            contest.standings.len(),
            contest.id,
            contest.name
        );
        simulate_contest(&mut players, &contest, &*system, 1500., 300.);
        last_contest_time = contest.time_seconds;
    }
    print_ratings(&players, last_contest_time - 183 * 86_400);
}
