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

    // Due to the Sized bound, in order to call this on trait objects containing `dyn Dataset`,
    // we would have to impl Dataset for each of these trait objects
    fn cached(self, cache_dir: impl AsRef<Path>) -> CachedDataset<Self>
    where
        Self: Sized,
    {
        std::fs::create_dir_all(&cache_dir).expect("Could not create cache directory");
        CachedDataset {
            base_dataset: self,
            cache_dir: cache_dir.as_ref().to_path_buf(),
        }
    }

    // IDK how to implement IntoIterator on Dataset, so this is the next best thing.
    // The return type must be a concrete type (either Box or custom DatasetIterator, not impl),
    // in case some impl Dataset overrides iter()
    fn iter(&self) -> Box<dyn Iterator<Item = Self::Item> + '_> {
        Box::new((0..self.len()).map(move |i| self.get(i)))
    }
}

// This function is just an example and will probably be erased.
// Non-trait functions can't be overridden, so they can statically dispatch an existential type
fn dataset_iter<T>(dataset: &dyn Dataset<Item = T>) -> impl Iterator<Item = T> + '_ {
    (0..dataset.len()).map(move |i| dataset.get(i))
}

// This function is just an example and will probably be erased.
// If this adaptor were a trait function, it would return a custom MapDataset type
fn dataset_map<'a, T, U: 'a, F: Fn(T) -> U + 'a>(
    dataset: &'a dyn Dataset<Item = T>,
    f: F,
) -> impl Dataset<Item = U> + 'a {
    ClosureDataset::new(dataset.len(), move |i| f(dataset.get(i)))
}

/// A dataset built from a closure
pub struct ClosureDataset<T, F: Fn(usize) -> T> {
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
pub struct CachedDataset<D: Dataset> {
    base_dataset: D,
    cache_dir: std::path::PathBuf,
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
pub struct CodeforcesDataset {
    contest_ids: Vec<usize>,
}

impl CodeforcesDataset {
    pub fn from_id_file(contest_id_file: impl AsRef<Path>) -> Self {
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

// Helper function to get data from the Codeforces API
pub fn get_dataset_from_codeforces_api() -> impl Dataset<Item = Contest> {
    CodeforcesDataset::from_id_file("../data/codeforces/contest_ids.json")
        .cached("../cache/codeforces")
}

// Helper function to get data that was manually added to the cache
pub fn get_dataset_from_disk<T: Serialize + DeserializeOwned>(
    dataset_dir: impl AsRef<Path>,
) -> impl Dataset<Item = T> {
    let ext = Some(std::ffi::OsStr::new("json"));
    let dataset_dir = dataset_dir.as_ref();
    let length = std::fs::read_dir(dataset_dir)
        .unwrap_or_else(|_| panic!("There's no dataset at {:?}", dataset_dir))
        .filter(|file| file.as_ref().unwrap().path().extension() == ext)
        .count();
    println!("Found {} json files at {:?}", length, dataset_dir);

    ClosureDataset::new(length, |i| {
        panic!("Expected to find contest {} in the cache, but didn't", i)
    })
    .cached(dataset_dir)
}

// Helper function to get any named dataset
// TODO: actually throw errors when the directory is not found
pub fn get_dataset_by_name(dataset_name: &str) -> Result<Box<dyn Dataset<Item = Contest>>, String> {
    if dataset_name == "codeforces" {
        Ok(Box::new(get_dataset_from_codeforces_api()))
    } else {
        let dataset_dir = format!("../cache/{}", dataset_name);
        Ok(Box::new(get_dataset_from_disk(dataset_dir)))
    }
}
