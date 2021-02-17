use super::{ContestSummary, PlayerEvent};
use multi_skill::summary::get_display_rating;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HistoryPoint {
    pub display_rating: i32,
    pub perf_score: i32,
    pub place: usize,
    pub num_contestants: usize,
    pub contest_name: String,
    pub contest_url: Option<String>,
    pub time_seconds: u64,
}

impl HistoryPoint {
    pub fn new(event: &PlayerEvent, contest: &ContestSummary) -> Self {
        Self {
            display_rating: get_display_rating(event),
            perf_score: event.perf_score,
            place: event.place,
            num_contestants: contest.num_contestants,
            contest_name: contest.name.clone(),
            contest_url: contest.url.clone(),
            time_seconds: contest.time_seconds,
        }
    }
}
