mod cf_api;
mod dataset;

pub use dataset::{get_dataset_from_disk, subrange, CachedDataset, ClosureDataset, Dataset};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;

fn one() -> f64 {
    1.0
}

fn is_one(&weight: &f64) -> bool {
    weight == one()
}

/// Represents the outcome of a contest.
#[derive(Serialize, Deserialize)]
pub struct Contest {
    /// A human-readable title for the contest.
    pub name: String,
    /// The source URL, if any.
    pub url: Option<String>,
    /// The relative weight of a contest, default is 1.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub weight: f64,
    /// The number of seconds from the Unix Epoch to the end of the contest.
    pub time_seconds: u64,
    /// The list of standings, containing a name and the enclosing range of ties.
    pub standings: Vec<(String, usize, usize)>,
}

impl Contest {
    /// Create a contest with empty standings, useful for testing.
    pub fn new(index: usize) -> Self {
        Self {
            name: format!("Round #{}", index),
            url: None,
            weight: 1.,
            time_seconds: index as u64 * 86_400,
            standings: vec![],
        }
    }

    /// Returns the contestant's position, if they participated.
    pub fn find_contestant(&mut self, handle: &str) -> Option<usize> {
        self.standings.iter().position(|x| x.0 == handle)
    }

    /// Remove a contestant with the given handle, and return it if it exists.
    pub fn remove_contestant(&mut self, handle: &str) -> Option<(String, usize, usize)> {
        let pos = self.find_contestant(handle)?;
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

    /// Remove all contestants for whom keep returns false.
    pub fn filter_by_handle(&mut self, keep: impl Fn(&str) -> bool) {
        self.standings.retain(|(handle, _, _)| keep(handle));
        let len = self.standings.len();
        let mut lo = 0;
        while lo < len {
            let mut hi = lo;
            while hi + 1 < len && self.standings[lo].1 == self.standings[hi + 1].1 {
                hi += 1;
            }
            for (_, st_lo, st_hi) in &mut self.standings[lo..=hi] {
                *st_lo = lo;
                *st_hi = hi;
            }
            lo = hi + 1;
        }
    }

    /// Add a contestant with the given handle in last place.
    pub fn push_contestant(&mut self, handle: impl Into<String>) {
        let place = self.standings.len();
        self.standings.push((handle.into(), place, place));
    }
}

/// Compressed summary of a contest
#[derive(Serialize, Deserialize)]
pub struct ContestSummary {
    pub name: String,
    pub url: Option<String>,
    pub weight: f64,
    pub time_seconds: u64,
    pub num_contestants: usize,
}

impl ContestSummary {
    /// Returns a summary of the given contest, stripped of detailed standings
    pub fn new(contest: &Contest) -> Self {
        Self {
            name: contest.name.clone(),
            url: contest.url.clone(),
            weight: contest.weight,
            time_seconds: contest.time_seconds,
            num_contestants: contest.standings.len(),
        }
    }
}

fn write_to_json<T: Serialize + ?Sized>(
    value: &T,
    path: impl AsRef<Path>,
) -> Result<(), &'static str> {
    let cached_json = serde_json::to_string_pretty(&value).map_err(|_| "Serialization error")?;
    std::fs::write(path.as_ref(), cached_json).map_err(|_| "File writing error")
}

fn write_to_csv<T: Serialize>(values: &[T], path: impl AsRef<Path>) -> Result<(), &'static str> {
    let file = std::fs::File::create(path.as_ref()).map_err(|_| "Output file not found")?;
    let mut writer = csv::Writer::from_writer(file);
    values
        .iter()
        .try_for_each(|val| writer.serialize(val))
        .map_err(|_| "Failed to serialize row")
}

pub fn write_slice_to_file<T: Serialize>(values: &[T], path: impl AsRef<Path>) {
    let path = path.as_ref();
    let write_res = match path.extension().and_then(|s| s.to_str()) {
        Some("json") => write_to_json(values, path),
        Some("csv") => write_to_csv(values, path),
        _ => Err("Invalid or missing filename extension"),
    };
    match write_res {
        Ok(()) => tracing::info!("Successfully wrote to {:?}", path),
        Err(msg) => tracing::error!("WARNING: failed write to {:?} because {}", path, msg),
    };
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

        assert_eq!(first_contest.weight, 1.);
        assert_eq!(first_contest.standings.len(), 66);
        assert_eq!(first_winner.0, "vepifanov");
        assert_eq!(first_winner.1, 0);
        assert_eq!(first_winner.2, 0);
    }
}
