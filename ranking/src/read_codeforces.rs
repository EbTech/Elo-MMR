use std::fs::File;
use std::io;
use std::str;
use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "status")]
enum CodeforcesPacket<T> {
    OK { result: T },
    FAILED { comment: String },
}

/// Result of API call: https://codeforces.com/apiHelp/methods#contest.ratingChanges
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct RatingChange {
    contestId: u64,
    contestName: String,
    handle: String,
    rank: u64,
    ratingUpdateTimeSeconds: u64,
    oldRating: i64,
    newRating: i64,
}

struct Scanner<R> {
    reader: R,
    buffer: Vec<String>,
}

impl<R: io::BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: vec![],
        }
    }

    fn token<T: str::FromStr>(&mut self) -> T {
        loop {
            if let Some(token) = self.buffer.pop() {
                return token.parse().ok().expect("Failed parse");
            }
            let mut input = String::new();
            self.reader.read_line(&mut input).expect("Failed read");
            self.buffer = input.split_whitespace().rev().map(String::from).collect();
        }
    }
}

fn scanner_from_file(filename: &str) -> Scanner<io::BufReader<std::fs::File>> {
    let file = File::open(filename).expect("Input file not found");
    Scanner::new(io::BufReader::new(file))
}

pub fn get_contests() -> Vec<usize> {
    let mut team_contests = HashSet::new();
    let mut solo_contests = Vec::new();

    let mut scan = scanner_from_file("../data/team_contests.txt");
    for _ in 0..scan.token::<usize>() {
        let contest = scan.token::<usize>();
        team_contests.insert(contest);
    }

    scan = scanner_from_file("../data/all_contests.txt");
    for _ in 0..scan.token::<usize>() {
        let contest = scan.token::<usize>();
        if !team_contests.contains(&contest) {
            solo_contests.push(contest);
        }
    }

    assert_eq!(team_contests.len(), 17);
    assert_eq!(solo_contests.len(), 1039);
    solo_contests
}

pub fn read_results(contest: usize) -> (String, Vec<(String, usize, usize)>) {
    /*
    TODO: Replace the current reading code to instead make all Codeforces API calls in parallel.
          Then erase all the Scanner stuff above. Here's a non-parallel example:

    let url = format!("https://codeforces.com/api/contest.ratingChanges?contestId={}", contest);
    let response = reqwest::blocking::get(&url).expect("HTTP error");
    let packet: CodeforcesPacket<Vec<RatingChange>> = response.json().expect("JSON parsing error");
    */

    let filename = format!("../standings/{}.txt", contest);
    let mut scan = scanner_from_file(&filename);
    let num_contestants = scan.token::<usize>();
    let title = scan.buffer.drain(..).rev().collect::<Vec<_>>().join(" ");

    let mut seen_handles = HashMap::with_capacity(num_contestants);
    let results: Vec<(String, usize, usize)> = (0..num_contestants)
        .map(|i| {
            let handle = scan.token::<String>();
            let rank_lo = scan.token::<usize>() - 1;
            let rank_hi = scan.token::<usize>() - 1;

            assert!(rank_lo <= i && i <= rank_hi && rank_hi < num_contestants);
            if let Some(j) = seen_handles.insert(handle.clone(), i) {
                // A hack to deal with unexplained duplicate contestants
                if contest == 447 || contest == 472 || contest == 615 {
                    println!(
                        "WARNING: contest {} has duplicate user {} at ranks {} and {}",
                        contest,
                        handle,
                        j + 1,
                        i + 1
                    );
                    let handle = handle + "_clone";
                    assert!(seen_handles.insert(handle.clone(), i).is_none());
                    return (handle, rank_lo, rank_hi);
                }

                panic!(
                    "Contest {} has duplicate user {} at ranks {} and {}",
                    contest,
                    handle,
                    j + 1,
                    i + 1
                );
            }
            (handle, rank_lo, rank_hi)
        })
        .collect();

    (title, results)
}