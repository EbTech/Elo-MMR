use multi_skill::data_processing::{
    get_dataset_by_name, subrange, write_slice_to_file, Contest, ContestSummary, Dataset,
};
use std::cmp::Reverse;
use std::collections::HashMap;

fn summarize(dataset: &dyn Dataset<Item = Contest>) -> (Vec<ContestSummary>, Vec<String>) {
    // Simulate the contests and rating updates
    let mut summaries = vec![];
    let mut participation_count = HashMap::<String, usize>::new();
    for contest in dataset.iter() {
        summaries.push(ContestSummary::new(&contest));
        for (handle, _, _) in &contest.standings {
            *participation_count.entry(handle.clone()).or_default() += 1;
        }
    }
    let mut sorted_names: Vec<_> = participation_count.keys().cloned().collect();
    sorted_names.sort_unstable_by_key(|name| Reverse(participation_count[name]));
    (summaries, sorted_names)
}

/// Sorts all players in descending order of how many contests they participated in
fn main() {
    tracing_subscriber::fmt::init();

    // Parse arguments and prepare the dataset
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 && args.len() != 3 {
        tracing::error!("Usage: {} dataset_name [num_contests]", args[0]);
        return;
    }
    let mut dataset = get_dataset_by_name(&args[1]).unwrap();
    if let Some(num_contests) = args.get(2).and_then(|s| s.parse().ok()) {
        dataset = Box::new(subrange(dataset, 0..num_contests));
    }

    let (summaries, sorted_names) = summarize(&dataset);

    let dir = std::path::PathBuf::from("../data/output");

    // Write contest summaries to data/codeforces/summaries.csv
    let summary_file = dir.join("all_contests.csv");
    write_slice_to_file(&summaries, &summary_file);

    // Write contest summaries to data/codeforces/summaries.csv
    let experienced_players_file = dir.join("experienced_players.csv");
    write_slice_to_file(&sorted_names, &experienced_players_file);
}
