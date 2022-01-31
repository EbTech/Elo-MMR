use multi_skill::data_processing::{
    get_dataset_by_name, try_write_slice_to_file, ContestDataset, ContestSummary, Dataset,
};
use std::cmp::Reverse;
use std::collections::HashMap;

fn summarize(dataset: &ContestDataset) -> (Vec<ContestSummary>, Vec<String>, usize, f64, usize) {
    // Simulate the contests and rating updates
    let mut summaries = vec![];
    let mut p_count = HashMap::<String, usize>::new();
    let mut p_min = usize::MAX;
    let mut p_max = 0;
    let mut p_total = 0;
    for contest in dataset.iter() {
        summaries.push(ContestSummary::new(&contest));
        for (handle, _, _) in &contest.standings {
            *p_count.entry(handle.clone()).or_default() += 1;
        }

        let participations = contest.standings.len();
        p_min = p_min.min(participations);
        p_max = p_max.max(participations);
        p_total += participations;
    }

    let mut sorted_names: Vec<_> = p_count.keys().cloned().collect();
    sorted_names.sort_unstable_by_key(|name| Reverse(p_count[name]));
    let p_mean = p_total as f64 / dataset.len() as f64;

    (summaries, sorted_names, p_min, p_mean, p_max)
}

fn main() {
    tracing_subscriber::fmt::init();

    // Parse arguments and prepare the dataset
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 && args.len() != 3 {
        tracing::error!("Usage: {} dataset_name [num_contests]", args[0]);
        return;
    }
    let dataset_name = &args[1];
    let mut dataset = get_dataset_by_name(dataset_name).unwrap();
    if let Some(num_contests) = args.get(2).and_then(|s| s.parse().ok()) {
        if num_contests > dataset.len() {
            tracing::error!(
                "Requested {} contests, but {} has only {}.",
                num_contests,
                args[1],
                dataset.len()
            );
        } else {
            dataset = dataset.subrange(0..num_contests).boxed();
        }
    }

    let (summaries, sorted_names, p_min, p_mean, p_max) = summarize(&dataset);

    tracing::info!("Number of contests = {}", dataset.len());
    tracing::info!("Number of users = {}", sorted_names.len());
    tracing::info!("Mean users per contest = {}", p_mean);
    tracing::info!("Min users = {}, Max users = {}", p_min, p_max);

    let dir = std::path::PathBuf::from("../data").join(dataset_name);
    std::fs::create_dir_all(&dir).expect("Could not create directory");

    // Write contest summaries to data/{source}/all_contests.csv
    let summary_file = dir.join("all_contests.csv");
    try_write_slice_to_file(&summaries, &summary_file);

    // Sort players in descending order of experience in data/{source}/names_by_experience.csv
    let experienced_players_file = dir.join("names_by_experience.csv");
    try_write_slice_to_file(&sorted_names, &experienced_players_file);
}
