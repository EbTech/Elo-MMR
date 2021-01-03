mod cf_api;
mod dataset;

pub use dataset::{get_dataset_from_disk, Dataset};
use serde::{Deserialize, Serialize};

/// Represents the outcome of a contest
#[derive(Serialize, Deserialize)]
pub struct Contest {
    pub id: usize,
    pub name: String,
    pub time_seconds: u64,
    pub standings: Vec<(String, usize, usize)>,
}

// Helper function to get data from the Codeforces API
pub fn get_dataset_from_codeforces_api(
    contest_id_file: impl AsRef<std::path::Path>,
) -> impl Dataset<Item = Contest> {
    let contests_json =
        std::fs::read_to_string(contest_id_file).expect("Failed to read contest IDs");
    let contest_ids: Vec<usize> =
        serde_json::from_str(&contests_json).expect("Failed to parse contest IDs as JSON");

    dataset::ClosureDataset::new(contest_ids.len(), move |i| {
        cf_api::fetch_cf_contest(contest_ids[i])
    })
}

// Helper function to get any named dataset
// TODO: actually throw errors when the directory is not found
pub fn get_dataset_by_name(dataset_name: &str) -> Result<Box<dyn Dataset<Item = Contest>>, String> {
    const CF_IDS: &str = "../data/codeforces/contest_ids.json";

    let dataset_dir = format!("../cache/{}", dataset_name);
    Ok(if dataset_name == "codeforces" {
        Box::new(get_dataset_from_codeforces_api(CF_IDS).cached(dataset_dir))
    } else {
        Box::new(get_dataset_from_disk(dataset_dir))
    })
}
