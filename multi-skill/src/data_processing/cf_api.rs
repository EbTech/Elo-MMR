use super::Contest;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

/// General response from the Codeforces API.
/// Codeforces documentation: https://codeforces.com/apiHelp
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE", tag = "status")]
enum CFResponse<T> {
    Ok { result: T },
    Failed { comment: String },
}

/// A RatingChange object from the Codeforces API.
/// Codeforces documentation: https://codeforces.com/apiHelp/objects#RatingChange
#[derive(Serialize, Deserialize)]
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
                eprintln!(
                    "WARNING @ {}: Inconsistent contest times {} and {}",
                    id, time_seconds, change.rating_update_time_seconds
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
                eprintln!(
                    "WARNING @ {}: duplicate user {} at positions {} and {}",
                    id, change.handle, i, j
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

/// Retrieves the Codeforces contest with the given ID, or panics if the API call fails.
/// Codeforces documentation: https://codeforces.com/apiHelp/methods#contest.ratingChanges
pub fn fetch_cf_contest(client: &Client, contest_id: usize) -> Contest {
    let response = client
        .get(&codeforces_api_url(contest_id))
        .send()
        .expect("Connection error: is Codeforces.com down?")
        .error_for_status()
        .expect("Status error: is Codeforces.com down?");
    let packet: CFResponse<Vec<CFRatingChange>> = response
        .json()
        .expect("Codeforces API response doesn't match the expected JSON schema");
    match packet {
        CFResponse::Ok { result } => result
            .try_into()
            .expect("Failed to parse JSON response as a valid Contest"),
        CFResponse::Failed { comment } => panic!(comment),
    }
}
