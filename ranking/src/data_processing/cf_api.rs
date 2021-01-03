use super::Contest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;

/// General response from the Codeforces API
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "status")]
enum CFResponse<T> {
    OK { result: T },
    FAILED { comment: String },
}

/// API documentation: https://codeforces.com/apiHelp/methods#contest.ratingChanges
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct CFRatingChange {
    contestId: usize,
    contestName: String,
    handle: String,
    rank: usize,
    ratingUpdateTimeSeconds: u64,
    oldRating: i32,
    newRating: i32,
}

/// Check the integrity of our API response and convert it into a more convenient format
impl TryFrom<Vec<CFRatingChange>> for Contest {
    type Error = String;

    fn try_from(json_contest: Vec<CFRatingChange>) -> Result<Self, Self::Error> {
        let first_change = json_contest.get(0).ok_or("Empty standings")?;
        let id = first_change.contestId;
        let name = first_change.contestName.clone();
        let time_seconds = first_change.ratingUpdateTimeSeconds;

        let mut lo_rank = json_contest.len() + 1;
        let mut hi_rank = json_contest.len() + 1;
        let mut seen_handles = HashMap::with_capacity(json_contest.len());
        let mut standings = Vec::with_capacity(json_contest.len());

        for (i, mut change) in json_contest.into_iter().enumerate().rev() {
            if id != change.contestId {
                return Err(format!(
                    "Inconsistent contests ids {} and {}",
                    id, change.contestId
                ));
            }
            if name != change.contestName {
                return Err(format!(
                    "Inconsistent contest names {} and {}",
                    name, change.contestName
                ));
            }
            if time_seconds != change.ratingUpdateTimeSeconds {
                // I don't know why but contests 61,318,347,373,381,400,404,405
                // each contain one discrepancy, usually 4 hours late
                eprintln!(
                    "WARNING @ {}: Inconsistent contest times {} and {}",
                    id, time_seconds, change.ratingUpdateTimeSeconds
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
            id,
            name,
            time_seconds,
            standings,
        })
    }
}

/// Retrieve a contest with a particular ID. If there's a cached entry with the same name in the
/// json/ directly, that will be used. This way, you can process your own custom contests.
/// If there is no cached entry, this function will attempt to retrieve one from Codeforces.
pub fn fetch_cf_contest(contest_id: usize) -> Contest {
    let url = format!(
        "https://codeforces.com/api/contest.ratingChanges?contestId={}",
        contest_id
    );
    let response = reqwest::blocking::get(&url).expect("HTTP error: is Codeforces.com down?");
    let status = response.status();
    if !status.is_success() {
        eprintln!("HTTP response {}: is Codeforces.com down?", status);
    }
    let packet: CFResponse<Vec<CFRatingChange>> = response
        .json()
        .expect("Codeforces API response doesn't match the expected JSON schema");
    match packet {
        CFResponse::OK { result } => {
            TryFrom::try_from(result).expect("Failed to parse JSON response as a Contest")
        }
        CFResponse::FAILED { comment } => panic!(comment),
    }
}
