use super::{read_csv, write_csv, Contest};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

/// General response from the Codeforces API.
/// Codeforces documentation: https://codeforces.com/apiHelp
#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE", tag = "status")]
enum CFResponse<T> {
    Ok { result: T },
    Failed { comment: String },
}

/// A RatingChange object from the Codeforces API.
/// Codeforces documentation: https://codeforces.com/apiHelp/objects#RatingChange
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CFRatingChange {
    contest_id: usize,
    contest_name: String,
    handle: String,
    rank: usize,
    rating_update_time_seconds: u64,
    old_rating: i32,
    new_rating: i32,
}

fn codeforces_human_url(contest_id: usize) -> String {
    format!("https://codeforces.com/contest/{}/standings", contest_id)
}

fn codeforces_api_url(contest_id: usize) -> String {
    format!(
        "https://codeforces.com/api/contest.ratingChanges?contestId={}",
        contest_id
    )
}

impl TryFrom<Vec<CFRatingChange>> for Contest {
    type Error = String;

    /// Checks the integrity of our API response and convert it into a more convenient format.
    fn try_from(json_contest: Vec<CFRatingChange>) -> Result<Self, Self::Error> {
        let first_change = json_contest.get(0).ok_or("Empty standings")?;
        let id = first_change.contest_id;
        let name = first_change.contest_name.clone();
        let time_seconds = first_change.rating_update_time_seconds;

        let mut lo_rank = json_contest.len() + 1;
        let mut hi_rank = json_contest.len() + 1;
        let mut seen_handles = HashMap::with_capacity(json_contest.len());
        let mut standings = Vec::with_capacity(json_contest.len());

        for (i, mut change) in json_contest.into_iter().enumerate().rev() {
            if id != change.contest_id {
                return Err(format!(
                    "Inconsistent contests ids {} and {}",
                    id, change.contest_id
                ));
            }
            if name != change.contest_name {
                return Err(format!(
                    "Inconsistent contest names {} and {}",
                    name, change.contest_name
                ));
            }
            if time_seconds != change.rating_update_time_seconds {
                // I don't know why but contests 61,318,347,373,381,400,404,405
                // each contain one discrepancy, usually 4 hours late
                tracing::warn!(
                    "@ {}: Inconsistent contest times {} and {}",
                    id,
                    time_seconds,
                    change.rating_update_time_seconds
                );
            }
            while let Some(j) = seen_handles.insert(change.handle.clone(), i) {
                // I don't know why but contests 447,472,615 have duplicate users
                if !(id == 447 || id == 472 || id == 615) {
                    return Err(format!(
                        "Duplicate user {} at positions {} and {}",
                        change.handle, i, j
                    ));
                }
                tracing::warn!(
                    "@ {}: duplicate user {} at positions {} and {}",
                    id,
                    change.handle,
                    i,
                    j
                );
                change.handle += "_clone";
            }

            if lo_rank == change.rank {
                if !(lo_rank < i + 2 && i < hi_rank) {
                    return Err(format!(
                        "Position {} is not between ranks {} and {}",
                        i + 1,
                        lo_rank,
                        hi_rank
                    ));
                }
            } else {
                if !(change.rank < lo_rank && lo_rank == i + 2) {
                    return Err(format!("Invalid start of rank {}", lo_rank));
                }
                hi_rank = lo_rank;
                lo_rank = change.rank;
            }

            standings.push((change.handle, lo_rank - 1, hi_rank - 2));
        }
        standings.reverse();

        Ok(Self {
            name,
            url: Some(codeforces_human_url(id)),
            weight: 1.0,
            time_seconds,
            standings,
        })
    }
}

/// Retrieves metadata and rating changes from the Codeforces contest with the given ID,
/// or yields an error if the API call fails, typically due to the contest being unrated.
/// Unrated contests may return CFResponse::Failed with status 400, or plain empty standings.
/// This function panics if there are networking or parsing errors.
/// Codeforces documentation: https://codeforces.com/apiHelp/methods#contest.ratingChanges
pub fn fetch_cf_contest(client: &Client, contest_id: usize) -> Result<Contest, String> {
    let response = client
        .get(&codeforces_api_url(contest_id))
        .send()
        .expect("Connection error: is Codeforces.com down?");
    if response.status().as_u16() != 400 {
        // Status code 400 may come from an unrated contest,
        // so it should not trigger a panic.
        response
            .error_for_status_ref()
            .expect("Status error: is Codeforces.com down?");
    }
    let packet: CFResponse<Vec<CFRatingChange>> = response
        .json()
        .expect("Codeforces API response doesn't match the expected JSON schema");
    let result = match packet {
        CFResponse::Ok { result } => result,
        CFResponse::Failed { comment } => return Err(comment),
    };

    if result.is_empty() {
        Err("Empty standings".into())
    } else {
        let contest = result
            .try_into()
            .expect("Failed to parse JSON response as a valid Contest");
        Ok(contest)
    }
}

/// A Contest object from the Codeforces API.
/// Codeforces documentation: https://codeforces.com/apiHelp/objects#Contest
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CFContest {
    id: usize,
    name: String,
    phase: String,
    duration_seconds: u64,
    start_time_seconds: u64,
}

fn try_fetch_new_ids(client: &Client, last_id: usize) -> reqwest::Result<Vec<usize>> {
    let packet: CFResponse<Vec<CFContest>> = client
        .get("https://codeforces.com/api/contest.list")
        .send()?
        .error_for_status()?
        .json()?;
    let contests = match packet {
        CFResponse::Ok { result } => result,
        CFResponse::Failed { comment } => panic!("Bad API response: {}", comment),
    };
    let team_contests = HashSet::<usize>::from_iter([
        524, 532, 541, 562, 566, 639, 641, 643, 695, 771, 772, 773, 823, 923, 924, 925, 951,
    ]);

    let new_contest_ids_rev: Vec<usize> = contests
        .into_iter()
        .inspect(|contest| tracing::info!("Trying round {}", contest.id))
        .filter(|contest| contest.phase.as_str() == "FINISHED")
        .map(|contest| contest.id)
        .take_while(|&id| id != last_id)
        .filter(|&id| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            !team_contests.contains(&id) && fetch_cf_contest(client, id).is_ok()
        })
        .collect();
    Ok(new_contest_ids_rev)
}

/// Retrieves the IDs of all non-team rated Codeforces rounds.
/// Codeforces documentation: https://codeforces.com/apiHelp/methods#contest.list
pub fn fetch_cf_contest_ids(client: &Client) -> Vec<usize> {
    const CF_IDS_FILE: &str = "../data/codeforces/contest_ids.csv";
    let mut local_contest_ids = read_csv(CF_IDS_FILE).expect("Failed to read contest IDs file");
    let last_local_contest_id = *local_contest_ids.last().unwrap_or(&0);

    match try_fetch_new_ids(client, last_local_contest_id) {
        Ok(new_contest_ids_rev) => {
            tracing::info!(
                "Found {} new contest IDs: {:?}",
                new_contest_ids_rev.len(),
                new_contest_ids_rev
            );
            local_contest_ids.extend(new_contest_ids_rev.into_iter().rev());
            write_csv(&local_contest_ids, CF_IDS_FILE)
                .expect("Failed to write updated contest ids");
        }
        Err(err) => tracing::error!(
            "Is Codeforces down? Couldn't fetch IDs because {}",
            err.to_string()
        ),
    }

    local_contest_ids
}
