use crate::read_codeforces::fetch_cf_contest;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents the outcome of a contest
#[derive(Serialize, Deserialize)]
pub struct Contest {
    pub id: usize,
    pub name: String,
    pub time_seconds: u64,
    pub standings: Vec<(String, usize, usize)>,
}

#[allow(dead_code)]
pub enum ContestSource {
    Codeforces,
    Reddit,
    StackOverflow,
    Synthetic,
}

pub struct ContestConfig {
    pub contest_id_file: String,
    pub contest_cache_folder: String,
}

pub fn get_contest_config(source: ContestSource) -> ContestConfig {
    let source_name = match source {
        ContestSource::Codeforces => "codeforces",
        ContestSource::Reddit => "reddit",
        ContestSource::StackOverflow => "stackoverflow",
        ContestSource::Synthetic => "synthetic",
    };

    ContestConfig {
        contest_id_file: format!("../data/{}/contest_ids.json", source_name),
        contest_cache_folder: format!("../cache/{}", source_name),
    }
}

/// Get a list of all the contest IDs in chronological order
pub fn get_contest_ids<P: AsRef<Path>>(contest_id_file: &P) -> Vec<usize> {
    let contests_json =
        std::fs::read_to_string(contest_id_file).expect("Failed to read contest IDs");
    serde_json::from_str(&contests_json).expect("Failed to parse contest IDs as JSON")
}

pub fn get_contest<P: AsRef<Path>>(cache_dir: P, contest_id: usize) -> Contest {
    let cache_file = cache_dir.as_ref().join(format!("{}.json", contest_id));
    // Try to read the contest from the cache
    match std::fs::read_to_string(&cache_file) {
        Ok(cached_json) => serde_json::from_str(&cached_json).expect("Failed to read cache"),
        Err(_) => {
            // The contest doesn't appear in our cache, so request it from the Codeforces API
            // TODO: eventually define dataset-specific getters
            let contest = fetch_cf_contest(contest_id);

            // Write the contest to the cache
            let cached_json = serde_json::to_string_pretty(&contest).expect("Serialization error");
            std::fs::write(&cache_file, cached_json).expect("Failed to write to cache");

            contest
        }
    }
}
