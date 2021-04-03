use super::Contest;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

const CURRENT_YEAR: usize = 2021;

/// A contest's representation in the CTFtime API,
/// as documented at https://ctftime.org/api
#[derive(Deserialize)]
struct CTFContest {
    title: String,
    scores: Vec<CTFPlace>,
    time: f64,
}

/// An individual place in the standings
#[derive(Deserialize)]
struct CTFPlace {
    team_id: usize,
    points: String,
    place: usize,
}

fn ctftime_human_url(contest_id: usize) -> String {
    format!("https://ctftime.org/event/{}", contest_id)
}

impl TryFrom<(usize, CTFContest)> for Contest {
    type Error = String;

    /// Checks the integrity of our API response and convert it into a more convenient format.
    fn try_from((id, json_contest): (usize, CTFContest)) -> Result<Self, Self::Error> {
        let len = json_contest.scores.len();
        let mut seen_handles = HashMap::with_capacity(len);
        let mut standings = Vec::with_capacity(len);

        for (i, place) in json_contest.scores.into_iter().enumerate() {
            let mut name = place.team_id.to_string();
            while let Some(j) = seen_handles.insert(name.clone(), i) {
                tracing::warn!(
                    "@ {}: duplicate team {} at positions {} and {}",
                    id,
                    name,
                    i,
                    j
                );
                name += "_clone";
            }
            standings.push((name, i, i));
        }

        Ok(Self {
            name: json_contest.title,
            url: Some(ctftime_human_url(id)),
            weight: 1.0,
            time_seconds: json_contest.time.round() as u64,
            standings,
        })
    }
}

// The preprocessing here is extremely slow and defeats the purpose of caching;
// ideally, the CTFtime API should expose the standings on a per-round basis.
// We should also provide human-readable team names. (but as a primary key?)
// Should the the backend's {handle}.{ext} provide a URL for the member/team?
pub fn fetch_ctf_history() -> Vec<Contest> {
    let client = Client::new();
    let mut contests: Vec<Contest> = (2011..=CURRENT_YEAR)
        .flat_map(|year| {
            let response = client
                .get(format!("https://ctftime.org/api/v1/results/{}/", year))
                .send()
                .expect("Connection error: is CTFtime.org down?")
                .error_for_status()
                .expect("Status error: is CTFtime.org  down?");
            let packet: HashMap<usize, CTFContest> = response
                .json()
                .expect("CTFtime API response doesn't match the expected JSON schema");
            packet
                .into_iter()
                .map(|entry| entry.try_into().expect("failed conversion"))
        })
        .collect();
    contests.sort_by_key(|contest| contest.time_seconds);
    contests
}
