use serde::{de::DeserializeOwned, Serialize};
use std::ops::{Bound, RangeBounds};
use std::path::{Path, PathBuf};

/// Generic `Dataset` trait, modeled after PyTorch's `utils.data.Dataset`.
/// It represents a collection of objects indexed in the range `0..len()`.
pub trait Dataset {
    /// The type of objects procured by the `Dataset`.
    type Item;
    /// The number of objects in the `Dataset`.
    fn len(&self) -> usize;
    /// Get the `index`'th element, where `0 <= index < len()`
    fn get(&self, index: usize) -> Self::Item;

    /// Whether this `Dataset` is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Modifies the dataset to check a cache directory before reading.
    /// If the cache entry is present, it's used instead of the underlying `get()`.
    /// If the cache entry is absent, it will be created after calling `get()`.
    // Due to the `Sized` bound, calling `cached()` on `dyn Dataset` trait objects
    // requires the `impl Dataset` implementations below, for `&T` and `Box T`.
    // If we wanted to avoid this complication, `CachedDataset` could have simply stored
    // a `Box<dyn Dataset>`, at the expense of a pointer indirection per method call.
    // Basically, our optimization allows `CachedDataset` to store `Dataset`s by value
    // (and statically dispatch its methods) or by pointer (with dynamic dispatch), as needed.
    fn cached(self, cache_dir: impl Into<PathBuf>) -> CachedDataset<Self>
    where
        Self: Sized,
    {
        let cache_dir = cache_dir.into();
        std::fs::create_dir_all(&cache_dir).expect("Could not create cache directory");
        CachedDataset {
            base_dataset: self,
            cache_dir,
        }
    }

    /// Produces an `Iterator` that produces the entire `Dataset` in indexed order.
    // I don't know how to implement `IntoIterator` on `Dataset`, so this is the next best thing.
    // The return type must be a concrete type (either `Box` or custom `DatasetIterator`, not `impl`),
    // in case some `impl Dataset` overrides `iter()`.
    fn iter(&self) -> Box<dyn Iterator<Item = Self::Item> + '_> {
        Box::new((0..self.len()).map(move |i| self.get(i)))
    }
}

/// A slice can act as an in-memory `Dataset`.
impl<T: Clone> Dataset for [T] {
    type Item = T;

    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> T {
        self[index].clone()
    }
}

/// References to `Dataset`s are also `Dataset`s.
impl<D: Dataset + ?Sized> Dataset for &D {
    type Item = D::Item;

    fn len(&self) -> usize {
        (**self).len()
    }

    fn get(&self, index: usize) -> Self::Item {
        (**self).get(index)
    }
}

impl<D: Dataset + ?Sized> Dataset for Box<D> {
    type Item = D::Item;

    fn len(&self) -> usize {
        (**self).len()
    }

    fn get(&self, index: usize) -> Self::Item {
        (**self).get(index)
    }
}

/// A `Dataset` defined in terms of a closure, which acts as a "getter".
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

/// A `Dataset` that uses a disk directory as its cache, useful when calls to `get()` are expensive.
/// Created using `Dataset::cached()`.
pub struct CachedDataset<D: Dataset> {
    base_dataset: D,
    cache_dir: PathBuf,
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
                super::write_to_json(&contest, &cache_file).expect("Failed to write to cache");
                tracing::info!("Codeforces contest successfully cached at {:?}", cache_file);

                contest
            }
        }
    }
}

/// Helper function to get data that is already stored inside a disk directory.
pub fn get_dataset_from_disk<T: Serialize + DeserializeOwned>(
    dataset_dir: impl AsRef<Path>,
) -> impl Dataset<Item = T> {
    // Check that the directory exists and count the number of JSON files
    let ext = Some(std::ffi::OsStr::new("json"));
    let dataset_dir = dataset_dir.as_ref();
    let length = std::fs::read_dir(dataset_dir)
        .unwrap_or_else(|_| panic!("There's no dataset at {:?}", dataset_dir))
        .filter(|file| file.as_ref().unwrap().path().extension() == ext)
        .count();
    tracing::info!("Found {} JSON files at {:?}", length, dataset_dir);

    // Every entry should already be in the directory; if not, we should panic
    ClosureDataset::new(length, |i| {
        panic!("Expected to find contest {} in the cache, but didn't", i)
    })
    .cached(dataset_dir)
}

/// Truncate a dataset to a given range
pub fn subrange<T>(
    dataset: impl Dataset<Item = T>,
    range: impl RangeBounds<usize>,
) -> impl Dataset<Item = T> {
    let start = match range.start_bound() {
        Bound::Included(&i) => i,
        Bound::Excluded(&i) => i + 1,
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&i) => i + 1,
        Bound::Excluded(&i) => i,
        Bound::Unbounded => dataset.len(),
    };
    assert!(start <= end);
    assert!(end <= dataset.len());
    let len = end - start;

    ClosureDataset::new(len, move |i| dataset.get(start + i))
}

/// Element-wise transform
pub fn map<T, U>(dataset: impl Dataset<Item = T>, f: impl Fn(T) -> U) -> impl Dataset<Item = U> {
    ClosureDataset::new(dataset.len(), move |i| f(dataset.get(i)))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_in_memory_dataset() {
        let vec = vec![5.7, 9.2, -1.5];
        let dataset: Box<dyn Dataset<Item = f64>> = Box::new(vec.as_slice());

        assert_eq!(dataset.len(), vec.len());
        for (data_val, &vec_val) in dataset.iter().zip(vec.iter()) {
            assert_eq!(data_val, vec_val);
        }
    }

    #[test]
    fn test_closure_dataset() {
        let dataset = ClosureDataset::new(10, |x| x * x);

        for (idx, val) in dataset.iter().enumerate() {
            assert_eq!(val, idx * idx);
        }
    }

    #[test]
    fn test_cached_dataset() {
        let length = 5;
        let cache_dir = "temp_dir_containing_squares";
        let cache = || std::fs::read_dir(cache_dir);
        let fancy_item = |idx: usize| (idx.checked_sub(2), vec![idx * idx; idx]);

        // Create a new directory
        assert!(cache().is_err());
        let data_from_fn = ClosureDataset::new(length, fancy_item).cached(cache_dir);

        // Write into both a Vec and an empty directory
        assert_eq!(cache().unwrap().count(), 0);
        let data_into_vec = data_from_fn.iter().collect::<Vec<_>>();

        // Read from a filled directory
        assert_eq!(cache().unwrap().count(), length);
        let data_from_disk = get_dataset_from_disk(cache_dir);

        // Check all three views into the data for correctness
        assert_eq!(data_from_fn.len(), length);
        assert_eq!(data_into_vec.len(), length);
        assert_eq!(data_from_disk.len(), length);
        for idx in 0..length {
            let expected = fancy_item(idx);
            let data_from_disk_val: (Option<usize>, Vec<usize>) = data_from_disk.get(idx);
            assert_eq!(data_from_fn.get(idx), expected);
            assert_eq!(data_into_vec[idx], expected);
            assert_eq!(data_from_disk_val, expected);
        }

        // Trash the directory
        assert_eq!(cache().unwrap().count(), length);
        std::fs::remove_dir_all(cache_dir).unwrap();
        assert!(cache().is_err());
    }
}
