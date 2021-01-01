use crate::read_codeforces::fetch_cf_contest;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::Path;

/// Represents the outcome of a contest
#[derive(Serialize, Deserialize)]
pub struct Contest {
    pub id: usize,
    pub name: String,
    pub time_seconds: u64,
    pub standings: Vec<(String, usize, usize)>,
}

/// Generic dataset trait, modeled after PyTorch's utils.data.Dataset
pub trait Dataset {
    type Item;
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> Self::Item;
}

/// A dataset built from a closure
struct ClosureDataset<T, F: Fn(usize) -> T> {
    length: usize,
    closure: F,
}

impl<T, F: Fn(usize) -> T> ClosureDataset<T, F> {
    pub fn new(length: usize, closure: F) -> Self {
        Self { length, closure }
    }
}

impl<T, F: Fn(usize) -> T> Dataset for ClosureDataset<T, F> {
    type Item = T;

    fn len(&self) -> usize {
        self.length
    }

    fn get(&self, index: usize) -> T {
        (self.closure)(index)
    }
}

/// A cached version of the base dataset
struct CachedDataset<D: Dataset> {
    base_dataset: D,
    cache_dir: std::path::PathBuf,
}

impl<D: Dataset> CachedDataset<D> {
    pub fn new(base_dataset: D, cache_dir: impl AsRef<Path>) -> Self {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        Self {
            base_dataset,
            cache_dir,
        }
    }
}

impl<D: Dataset> Dataset for CachedDataset<D>
where
    D::Item: Serialize + DeserializeOwned,
{
    type Item = D::Item;

    fn len(&self) -> usize {
        self.base_dataset.len()
    }

    fn get(&self, index: usize) -> Self::Item {
        let cache_file = self.cache_dir.join(format!("{}.json", index));
        // Try to read the contest from the cache
        match std::fs::read_to_string(&cache_file) {
            Ok(cached_json) => serde_json::from_str(&cached_json).expect("Failed to read cache"),
            Err(_) => {
                // The contest doesn't appear in our cache, so request it from the base dataset
                let contest = self.base_dataset.get(index);

                // Write the contest to the cache
                let cached_json =
                    serde_json::to_string_pretty(&contest).expect("Serialization error");
                std::fs::write(&cache_file, cached_json).expect("Failed to write to cache");
                println!("Codeforces contest successfully cached at {:?}", cache_file);

                contest
            }
        }
    }
}

/// Codeforces dataset
struct CodeforcesDataset {
    contest_ids: Vec<usize>,
}

impl CodeforcesDataset {
    pub fn new(contest_id_file: impl AsRef<Path>) -> Self {
        let contests_json =
            std::fs::read_to_string(contest_id_file).expect("Failed to read contest IDs");
        let contest_ids =
            serde_json::from_str(&contests_json).expect("Failed to parse contest IDs as JSON");
        Self { contest_ids }
    }
}

impl Dataset for CodeforcesDataset {
    type Item = Contest;

    fn len(&self) -> usize {
        self.contest_ids.len()
    }

    fn get(&self, index: usize) -> Contest {
        fetch_cf_contest(self.contest_ids[index])
    }
}

// Helper function to get data that was manually added to the cache
pub fn get_cached_dataset(
    cache_dir: impl AsRef<Path>,
    num_rounds: usize,
) -> impl Dataset<Item = Contest> {
    let panic_dataset = ClosureDataset::new(num_rounds, |i| {
        panic!("Expected to find contest {} in the cache, but didn't", i)
    });
    CachedDataset::new(panic_dataset, cache_dir)
}

// Helper function to get data from the Codeforces API
pub fn get_codeforces_dataset() -> impl Dataset<Item = Contest> {
    let cf_dataset = CodeforcesDataset::new("../data/codeforces/contest_ids.json");
    CachedDataset::new(cf_dataset, "../cache/codeforces")
}

// Helper function to get any named dataset
pub fn get_dataset_by_name(dataset_name: &str) -> Result<Box<dyn Dataset<Item = Contest>>, String> {
    if dataset_name == "codeforces" {
        return Ok(Box::new(get_codeforces_dataset()));
    }

    // The non-Codeforces datasets are assumed to be stored in their entirety
    let ext = Some(std::ffi::OsStr::new("json"));
    let cache_dir = format!("../cache/{}", dataset_name);
    let num_contests = std::fs::read_dir(&cache_dir)
        .unwrap_or_else(|_| panic!("There's no dataset at {}", cache_dir))
        .filter(|file| file.as_ref().unwrap().path().extension() == ext)
        .count();

    println!("Found {} {}/*.json files", num_contests, cache_dir);
    Ok(Box::new(get_cached_dataset(cache_dir, num_contests)))
}

// IDK how to implement IntoIterator on Dataset, so this is the next best thing
pub fn iterate_data<T>(dataset: &dyn Dataset<Item = T>) -> impl Iterator<Item = T> + '_ {
    (0..dataset.len()).map(move |i| dataset.get(i))
}
