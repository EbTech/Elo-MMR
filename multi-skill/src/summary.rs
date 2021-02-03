use crate::systems::Player;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

const NUM_TITLES: usize = 11;
const TITLE_BOUND: [i32; NUM_TITLES] = [
    -999, 1000, 1200, 1400, 1600, 1800, 2000, 2200, 2400, 2700, 3000,
];
const TITLE: [&str; NUM_TITLES] = [
    "Ne", "Pu", "Ap", "Sp", "Ex", "CM", "Ma", "IM", "GM", "IG", "LG",
];

pub struct GlobalSummary {
    mean_rating: f64,
    title_count: Vec<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerSummary {
    rank: Option<usize>,
    cur_rating: i32,
    max_rating: i32,
    cur_sigma: i32,
    num_contests: usize,
    last_contest: usize,
    last_contest_time: u64,
    last_perf: i32,
    last_delta: i32,
    handle: String,
}

pub fn make_leaderboard(
    players: &HashMap<String, RefCell<Player>>,
    rated_since: u64,
) -> (GlobalSummary, Vec<PlayerSummary>) {
    let mut rating_data = Vec::with_capacity(players.len());
    let mut title_count = vec![0; NUM_TITLES];
    let sum_ratings = {
        let mut ratings: Vec<f64> = players
            .iter()
            .map(|(_, player)| player.borrow().approx_posterior.mu)
            .collect();
        ratings.sort_by(|a, b| a.partial_cmp(b).expect("NaN is unordered"));
        ratings.into_iter().sum::<f64>()
    };
    for (handle, player) in players {
        let player = player.borrow_mut();
        let num_contests = player.event_history.len();
        let last_event = player.event_history.last().unwrap();
        let max_rating = player
            .event_history
            .iter()
            .map(|event| event.display_rating)
            .max()
            .unwrap();
        let previous_rating = if num_contests == 1 {
            960 // TODO: get rid of this magic number
        } else {
            player.event_history[num_contests - 2].display_rating
        };
        rating_data.push(PlayerSummary {
            rank: None,
            cur_rating: last_event.display_rating,
            max_rating,
            cur_sigma: player.approx_posterior.sig.round() as i32,
            num_contests,
            last_contest: last_event.contest_id,
            last_contest_time: last_event.contest_time,
            last_perf: last_event.perf_score,
            last_delta: last_event.display_rating - previous_rating,
            handle: handle.clone(),
        });

        if last_event.contest_time > rated_since {
            if let Some(title_id) = (0..NUM_TITLES)
                .rev()
                .find(|&i| last_event.display_rating >= TITLE_BOUND[i])
            {
                title_count[title_id] += 1;
            }
        }
    }
    rating_data.sort_unstable_by_key(|data| (-data.cur_rating, data.handle.clone()));

    let mut rank = 0;
    for data in &mut rating_data {
        if data.last_contest_time > rated_since {
            rank += 1;
            data.rank = Some(rank);
        }
    }

    let global_summary = GlobalSummary {
        mean_rating: sum_ratings / players.len() as f64,
        title_count,
    };

    (global_summary, rating_data)
}

pub fn print_ratings(players: &HashMap<String, RefCell<Player>>, rated_since: u64) {
    let (summary, rating_data) = make_leaderboard(players, rated_since);

    let filename = "../data/ratings_output.csv";
    let file = std::fs::File::create(filename).expect("Output file not found");

    println!("Mean rating.mu = {}", summary.mean_rating);
    for i in (0..NUM_TITLES).rev() {
        println!(
            "{} {} x{:6}",
            TITLE_BOUND[i], TITLE[i], summary.title_count[i]
        );
    }
    println!("Detailed ratings saved to {}", filename);

    let mut writer = csv::Writer::from_writer(file);
    for data in rating_data {
        writer.serialize(data).unwrap();
    }
}
