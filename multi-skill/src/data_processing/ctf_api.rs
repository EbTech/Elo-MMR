use super::{CURRENT_YEAR, Contest};
use crate::systems::outcome_free;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;

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

/// Result of a query of teams
#[derive(Deserialize)]
struct CTFTeams {
    limit: usize,
    result: Vec<CTFTeam>,
    offset: usize,
}

/// Summary of a team
#[derive(Deserialize)]
struct CTFTeam {
    aliases: Vec<String>,
    country: String,
    academic: bool,
    id: usize,
    name: String,
}

fn ctftime_human_url(contest_id: usize) -> String {
    format!("https://ctftime.org/event/{}", contest_id)
}

impl TryFrom<(usize, CTFContest, &HashMap<usize, String>)> for Contest {
    type Error = String;

    /// Checks the integrity of our API response and convert it into a more convenient format.
    fn try_from(
        (id, json_contest, id_to_name): (usize, CTFContest, &HashMap<usize, String>),
    ) -> Result<Self, Self::Error> {
        let url = ctftime_human_url(id);
        let len = json_contest.scores.len();
        let mut seen_handles = HashMap::with_capacity(len);
        let mut standings = Vec::with_capacity(len);

        for (i, place) in json_contest.scores.into_iter().enumerate() {
            let mut name = match id_to_name.get(&place.team_id) {
                Some(name) => name.clone(),
                None => {
                    tracing::warn!("Team {} not found", place.team_id);
                    place.team_id.to_string()
                }
            };
            while let Some(j) = seen_handles.insert(name.clone(), i) {
                tracing::warn!(
                    "{} has duplicate team {} at positions {} and {}",
                    url,
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
            url: Some(url),
            rating_params: Default::default(),
            time_seconds: json_contest.time.round() as u64,
            standings,
        })
    }
}

// The preprocessing here is extremely slow and defeats the purpose of caching;
// ideally, the CTFtime API should expose the standings on a per-round basis.
// Currently, to use caching, we have to manually comment out the CTFtime lines
// from mod::get_dataset_by_name() after clearing & fetching the data once.
// We should also provide human-readable team names. (but as a primary key?)
// Should the the backend's {handle}.{ext} provide a URL for the member/team?
pub fn fetch_ctf_history() -> Vec<Contest> {
    let client = Client::new();
    let id_to_name = fetch_ctf_teams(&client);
    let mut contests: Vec<_> = (2011..=CURRENT_YEAR)
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
                .map(|(k, v)| (k, v, &id_to_name).try_into().expect("failed conversion"))
        })
        .filter(|contest: &Contest| !outcome_free(&contest.standings))
        .collect();
    contests.sort_by_key(|contest| contest.time_seconds);
    contests
}

pub fn fetch_ctf_teams(client: &Client) -> HashMap<usize, String> {
    const LIMIT: usize = 300; // the largest query allowed by the API
    let mut id_to_name = HashMap::new();
    for offset in (0..).step_by(LIMIT) {
        let response = client
            .get(format!(
                "https://ctftime.org/api/v1/teams/?limit={}&offset={}",
                LIMIT, offset
            ))
            .send()
            .expect("Connection error: is CTFtime.org down?")
            .error_for_status()
            .expect("Status error: is CTFtime.org down?");
        let packet: CTFTeams = response
            .json()
            .expect("CTFtime API response doesn't match the expected JSON schema");
        let end_of_list = packet.result.len() < LIMIT;
        for team in packet.result {
            id_to_name.insert(team.id, team.name);
        }
        if end_of_list {
            break;
        }
    }
    id_to_name
}
