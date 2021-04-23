use multi_skill::data_processing::{fetch_cf_contest_list, write_slice_to_file};

fn main() {
    tracing_subscriber::fmt::init();

    // Parse optional argument n, to get the last n contests
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 1 && args.len() != 2 {
        tracing::error!("Usage: {} [num_recent_contests]", args[0]);
        return;
    }
    let num_recent = args.get(1).and_then(|s| s.parse().ok());

    let client = reqwest::blocking::Client::new();
    let contest_ids = fetch_cf_contest_list(&client, num_recent);

    let ids_file = "../data/codeforces/contest_ids.csv";
    write_slice_to_file(&contest_ids, ids_file);
}
