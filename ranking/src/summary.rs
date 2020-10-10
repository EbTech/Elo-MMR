use crate::compute_ratings::Player;
use std::cell::RefCell;
use std::collections::HashMap;

const NUM_TITLES: usize = 11;
const TITLE_BOUND: [i32; NUM_TITLES] = [
    -999, 1000, 1200, 1400, 1600, 1800, 2000, 2200, 2400, 2700, 3000,
];
const TITLE: [&str; NUM_TITLES] = [
    "Ne", "Pu", "Ap", "Sp", "Ex", "CM", "Ma", "IM", "GM", "IG", "LG",
];

struct RatingData {
    cur_rating: i32,
    max_rating: i32,
    handle: String,
    last_contest: usize,
    last_contest_time: u64,
    last_perf: i32,
    last_delta: i32,
}

pub fn print_ratings(players: &HashMap<String, RefCell<Player>>, rated_since: u64) {
    let filename = "../data/CFratings_temp.txt";
    let file = std::fs::File::create(filename).expect("Output file not found");
    let mut out = std::io::BufWriter::new(file);

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
        let last_event = player.event_history.last().unwrap();
        let max_rating = player
            .event_history
            .iter()
            .map(|event| event.display_rating)
            .max()
            .unwrap();
        let last_perf = player
            .logistic_factors
            .back()
            .map(|r| r.mu.round() as i32)
            .unwrap_or(0);
        let previous_rating = if player.event_history.len() == 1 {
            1060 // TODO: get rid of this magic number
        } else {
            player.event_history[player.event_history.len() - 2].display_rating
        };
        rating_data.push(RatingData {
            cur_rating: last_event.display_rating,
            max_rating,
            handle: handle.clone(),
            last_contest: last_event.contest_id,
            last_contest_time: last_event.contest_time,
            last_perf,
            last_delta: last_event.display_rating - previous_rating,
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

    use std::io::Write;
    writeln!(
        out,
        "Mean rating.mu = {}",
        sum_ratings / players.len() as f64
    )
    .ok();

    for i in (0..NUM_TITLES).rev() {
        writeln!(out, "{} {} x{:6}", TITLE_BOUND[i], TITLE[i], title_count[i]).ok();
    }

    let mut rank = 0;
    for data in rating_data {
        if data.last_contest_time > rated_since {
            rank += 1;
            write!(out, "{:6}", rank).ok();
        } else {
            write!(out, "{:>6}", "-").ok();
        }
        write!(out, " {:4}({:4})", data.cur_rating, data.max_rating).ok();
        write!(out, " {:<26}contest/{:4}: ", data.handle, data.last_contest).ok();
        writeln!(
            out,
            "perf ={:5}, delta ={:4}",
            data.last_perf, data.last_delta
        )
        .ok();
    }
}
