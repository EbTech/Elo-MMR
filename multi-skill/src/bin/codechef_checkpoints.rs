use multi_skill::data_processing::{
    get_dataset_by_name, read_csv, try_write_slice_to_file, write_json,
};
use multi_skill::summary::make_leaderboard;
use multi_skill::systems::{simulate_contest, EloMMR, EloMMRVariant};

fn main() {
    tracing_subscriber::fmt::init();

    let dataset = get_dataset_by_name("codechef").unwrap();
    let mut mu_noob = 1500.;
    let sig_noob = 325.;
    let weight_limit = 0.2;
    let sig_limit = 75.;
    let system = EloMMR {
        weight_limit,
        sig_limit,
        drift_per_sec: 0.,
        split_ties: false,
        subsample_size: 512,
        subsample_bucket: 0.5,
        variant: EloMMRVariant::Logistic(0.1),
    };

    let mut players = std::collections::HashMap::new();

    // Get list of contest names to compare with Codechef's rating system
    let paths = std::fs::read_dir("/home/work_space/elommr-data/ratings").unwrap();
    let mut checkpoints = std::collections::HashSet::<String>::new();
    for path in paths {
        if let Some(contest_name) = path.unwrap().path().file_stem() {
            if let Ok(string_name) = contest_name.to_os_string().into_string() {
                checkpoints.insert(string_name);
            }
        }
    }

    // Run the contest histories and measure
    let dir = std::path::PathBuf::from("/home/work_space/elommr-data/elommr-checkpoints/codechef/");
    for (index, contest) in dataset.iter().enumerate() {
        tracing::debug!(
            "Processing\n{:6} contestants in{:5}th contest with wt={}: {}",
            contest.standings.len(),
            index,
            contest.weight,
            contest.name
        );

        // At some point, codechef changed the default rating!
        if contest.name == "START25B" {
            mu_noob = 1000.;
        }

        // Now run the actual rating update
        simulate_contest(&mut players, &contest, &system, mu_noob, sig_noob, index);

        if checkpoints.contains(&contest.name) {
            let output_file = dir.join(contest.name.clone() + ".csv");
            let (_summary, rating_data) = make_leaderboard(&players, 0);
            try_write_slice_to_file(&rating_data, &output_file);
        }
    }
}
