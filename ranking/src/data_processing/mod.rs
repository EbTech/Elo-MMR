mod cf_api;
mod dataset;

pub use dataset::{get_dataset_from_disk, CachedDataset, ClosureDataset, Dataset};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

fn one() -> f64 {
    1.0
}

fn is_one(&weight: &f64) -> bool {
    weight == one()
}

/// Represents the outcome of a contest.
#[derive(Serialize, Deserialize)]
pub struct Contest {
    /// A unique ID for the contest.
    pub id: usize,
    /// A human-readable title for the contest.
    pub name: String,
    /// The number of seconds from the Unix Epoch to the end of the contest.
    pub time_seconds: u64,
    /// The list of standings, containing a name and the enclosing range of ties.
    pub standings: Vec<(String, usize, usize)>,
    /// The relative weight of a contest, default is 1.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub weight: f64,
}

impl Contest {
    /// Create a contest with empty standings, useful for testing.
    pub fn with_id(id: usize) -> Self {
        Self {
            id,
            name: format!("Round #{}", id),
            time_seconds: id as u64 * 86_400,
            standings: vec![],
            weight: 1.,
        }
    }

    /// Remove a contestant with the given handle, and return it if it exists.
    pub fn remove_contestant(&mut self, handle: &str) -> Option<(String, usize, usize)> {
        let pos = self.standings.iter().position(|x| x.0 == handle)?;
        let contestant = self.standings.remove(pos);
        for (_, lo, hi) in self.standings.iter_mut() {
            if *hi >= pos {
                *hi -= 1;
                if *lo > pos {
                    *lo -= 1;
                }
            }
        }
        Some(contestant)
    }

    /// Add a contestant with the given handle in last place.
    pub fn push_contestant(&mut self, handle: impl Into<String>) {
        let place = self.standings.len();
        self.standings.push((handle.into(), place, place));
    }
}

/// Helper function to get contest results from the Codeforces API.
pub fn get_dataset_from_codeforces_api(
    contest_id_file: impl AsRef<std::path::Path>,
) -> impl Dataset<Item = Contest> {
    let client = Client::new();
    let contests_json =
        std::fs::read_to_string(contest_id_file).expect("Failed to read contest IDs from file");
    let contest_ids: Vec<usize> = serde_json::from_str(&contests_json)
        .expect("Failed to parse JSON contest IDs as a Vec<usize>");

    dataset::ClosureDataset::new(contest_ids.len(), move |i| {
        cf_api::fetch_cf_contest(&client, contest_ids[i])
    })
}

/// Helper function to get any named dataset.
// TODO: actually throw errors when the directory is not found.
pub fn get_dataset_by_name(
    dataset_name: &str,
) -> Result<Box<dyn Dataset<Item = Contest> + Send + Sync>, String> {
    const CF_IDS: &str = "../data/codeforces/contest_ids.json";

    let dataset_dir = format!("../cache/{}", dataset_name);
    Ok(if dataset_name == "codeforces" {
        Box::new(get_dataset_from_codeforces_api(CF_IDS).cached(dataset_dir))
    } else {
        Box::new(get_dataset_from_disk(dataset_dir))
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_codeforces_data() {
        let dataset = get_dataset_by_name("codeforces").unwrap();
        let first_contest = dataset.get(0);
        let first_winner = &first_contest.standings[0];

        assert_eq!(first_contest.id, 1);
        assert_eq!(first_contest.standings.len(), 66);
        assert_eq!(first_winner.0, "vepifanov");
        assert_eq!(first_winner.1, 0);
        assert_eq!(first_winner.2, 0);
    }
}
