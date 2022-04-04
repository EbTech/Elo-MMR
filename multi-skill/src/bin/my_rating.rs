// Copy-paste a spreadsheet column of CF handles as input to this program, then
// paste this program's output into the spreadsheet's ratings column.
use io::{BufRead, Write};
use multi_skill::data_processing::read_csv;
use multi_skill::summary::PlayerSummary;
use std::collections::HashMap;
use std::io;

fn main() {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        tracing::error!("Usage: {} dataset_name", args[0]);
        return;
    }

    let filename = format!("../data/{}/all_players.csv", args[1]);
    let players: Vec<PlayerSummary> = read_csv(filename, true).expect("File not found");
    let ratings: HashMap<String, i32> = players
        .into_iter()
        .map(|player| (player.handle, player.display_rating))
        .collect();

    println!("Type some handles on separate lines, followed by \"DONE\":");

    let (stdin, stdout) = (io::stdin(), io::stdout());
    let mut out = io::BufWriter::new(stdout.lock());
    for handle in stdin.lock().lines().map(|l| l.expect("Failed stdin read")) {
        if handle == "DONE" {
            break;
        }
        let rating = ratings.get(&handle).unwrap_or(&0);
        writeln!(out, "{}", rating).ok();
    }
}
