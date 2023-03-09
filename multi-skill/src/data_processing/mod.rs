mod cf_api;
mod ctf_api;
mod dataset;

pub use cf_api::fetch_cf_contest_ids;
pub use dataset::{get_dataset_from_disk, CachedDataset, ClosureDataset, Dataset, Wrap};
use rand::seq::SliceRandom;
use reqwest::blocking::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::Path;

pub const CURRENT_YEAR: usize = 2022;

fn one() -> f64 {
    1.
}

fn f64_max() -> f64 {
    f64::MAX
}

fn is_one(&weight: &f64) -> bool {
    weight == one()
}

fn is_f64_max(&num: &f64) -> bool {
    num == f64::MAX
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ContestRatingParams {
    /// The relative weight of a contest, default is 1.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub weight: f64,
    /// Maximum performance this contest is intended to measure, default is infinity.
    #[serde(default = "f64_max", skip_serializing_if = "is_f64_max")]
    pub perf_ceiling: f64,
}

impl Default for ContestRatingParams {
    fn default() -> Self {
        Self {
            weight: one(),
            perf_ceiling: f64_max(),
        }
    }
}

/// Represents the outcome of a contest.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Contest {
    /// A human-readable title for the contest.
    pub name: String,
    /// The source URL, if any.
    pub url: Option<String>,
    /// Parameters that adjust characteristics of rating systems
    #[serde(flatten)]
    pub rating_params: ContestRatingParams,
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
            rating_params: Default::default(),
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

    /// Assuming `self.standings` is a subset of a valid standings list,
    /// corrects the `lo` and `hi` values to make the new list valid
    fn fix_lo_hi(&mut self) {
        self.standings.sort_unstable_by_key(|(_, lo, _)| *lo);
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

    fn clone_with_standings(&self, standings: Vec<(String, usize, usize)>) -> Self {
        let mut contest = Self {
            name: self.name.clone(),
            url: self.url.clone(),
            rating_params: self.rating_params,
            time_seconds: self.time_seconds,
            standings,
        };
        contest.fix_lo_hi();
        contest
    }

    /// Split into random disjoint contests, each with at most n participants
    pub fn random_split<R: ?Sized + rand::Rng>(
        mut self,
        n: usize,
        rng: &mut R,
    ) -> impl Iterator<Item = Contest> {
        self.standings.shuffle(rng);
        let split_standings: Vec<_> = self.standings.chunks(n).map(<[_]>::to_vec).collect();
        split_standings
            .into_iter()
            .map(move |chunk| self.clone_with_standings(chunk))
    }

    /// Add a contestant with the given handle in last place.
    pub fn push_contestant(&mut self, handle: impl Into<String>) {
        let place = self.standings.len();
        self.standings.push((handle.into(), place, place));
    }
}

/// Compressed summary of a contest
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContestSummary {
    pub name: String,
    pub url: Option<String>,
    pub rating_params: ContestRatingParams,
    pub time_seconds: u64,
    pub num_contestants: usize,
}

impl ContestSummary {
    /// Returns a summary of the given contest, stripped of detailed standings
    pub fn new(contest: &Contest) -> Self {
        Self {
            name: contest.name.clone(),
            url: contest.url.clone(),
            rating_params: contest.rating_params,
            time_seconds: contest.time_seconds,
            num_contestants: contest.standings.len(),
        }
    }
}

pub fn read_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, String> {
    let json_str = std::fs::read_to_string(path.as_ref()).map_err(|e| e.to_string())?;
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}

pub fn read_csv<T: DeserializeOwned>(
    path: impl AsRef<Path>,
    has_headers: bool,
) -> csv::Result<Vec<T>> {
    csv::ReaderBuilder::new()
        .has_headers(has_headers)
        .from_path(path)?
        .deserialize()
        .collect()
}

pub fn write_json<T: Serialize + ?Sized>(value: &T, path: impl AsRef<Path>) -> Result<(), String> {
    let cached_json = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    std::fs::write(path.as_ref(), cached_json).map_err(|e| e.to_string())
}

pub fn write_csv<T: Serialize>(values: &[T], path: impl AsRef<Path>) -> csv::Result<()> {
    let file = std::fs::File::create(path.as_ref())?;
    let mut writer = csv::Writer::from_writer(file);
    values.iter().try_for_each(|val| writer.serialize(val))
}

pub fn write_slice_to_file<T: Serialize>(
    values: &[T],
    path: impl AsRef<Path>,
) -> Result<(), String> {
    let path = path.as_ref();
    match path.extension().and_then(|s| s.to_str()) {
        Some("json") => write_json(values, path),
        Some("csv") => write_csv(values, path).map_err(|e| e.to_string()),
        _ => Err("Invalid or missing filename extension".into()),
    }
}

pub fn try_write_json<T: Serialize + ?Sized>(value: &T, path: impl AsRef<Path>) {
    let path = path.as_ref();
    let write_res = write_json(value, path);
    match write_res {
        Ok(()) => tracing::info!("Successfully wrote to {:?}", path),
        Err(msg) => tracing::error!("WARNING: failed write to {:?} because {}", path, msg),
    };
}

pub fn try_write_slice_to_file<T: Serialize>(values: &[T], path: impl AsRef<Path>) {
    let path = path.as_ref();
    let write_res = write_slice_to_file(values, path);
    match write_res {
        Ok(()) => tracing::info!("Successfully wrote to {:?}", path),
        Err(msg) => tracing::error!("WARNING: failed write to {:?} because {}", path, msg),
    };
}

pub fn log_expected_error(msg: String, is_expected: bool) -> Result<(), String> {
    if is_expected {
        tracing::warn!("Expected error {}", msg);
        Ok(())
    } else {
        tracing::error!("Unexpected error {}", msg);
        Err(msg)
    }
}

/// Helper function to get contest results from the Codeforces API.
pub fn get_dataset_from_codeforces_api() -> Wrap<impl Dataset<Item = Contest>> {
    let client = Client::new();
    let contest_ids = fetch_cf_contest_ids(&client);

    Wrap::from_closure(contest_ids.len(), move |i| {
        cf_api::fetch_cf_contest(&client, contest_ids[i]).expect("Failed to fetch contest")
    })
}

/// Helper function to get contest results from the CTFtime API.
pub fn get_dataset_from_ctftime_api() -> Wrap<impl Dataset<Item = Contest>> {
    let contests = ctf_api::fetch_ctf_history();

    Wrap::from_closure(contests.len(), move |i| contests[i].clone())
}

pub type BoxedDataset<T> = Box<dyn Dataset<Item = T> + Send + Sync>;
pub type ContestDataset = Wrap<BoxedDataset<Contest>>;

/// Helper function to get any named dataset.
// TODO: actually throw errors when the directory is not found.
pub fn get_dataset_by_name(dataset_name: &str) -> Result<ContestDataset, String> {
    let dataset_dir = format!("../cache/{}", dataset_name);
    let dataset = if dataset_name == "codeforces" {
        // Rate-limit API calls so we don't burden Codeforces
        get_dataset_from_codeforces_api()
            .rate_limit(std::time::Duration::from_millis(500))
            .cached(dataset_dir)
            .boxed()
    //} else if dataset_name == "ctf" {
    //    get_dataset_from_ctftime_api().cached(dataset_dir).boxed()
    } else {
        get_dataset_from_disk(dataset_dir).boxed()
    };
    Ok(dataset)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_codeforces_data() {
        let dataset = get_dataset_by_name("codeforces").unwrap();
        let first_contest = dataset.get(0);
        let first_winner = &first_contest.standings[0];

        assert_eq!(first_contest.rating_params.weight, 1.);
        assert_eq!(first_contest.standings.len(), 66);
        assert_eq!(first_winner.0, "vepifanov");
        assert_eq!(first_winner.1, 0);
        assert_eq!(first_winner.2, 0);
    }
}
