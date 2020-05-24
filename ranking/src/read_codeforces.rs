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
    TODO: Replace the current reading code to instead use the JSON API.
    Then erase all the Scanner stuff above. Example:

    let contest_url = format!("https://codeforces.com/api/contest.ratingChanges?contestId={}", contest};
    let packet_as_json: &str = r#"{"status":"OK","result":[{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"vepifanov","rank":1,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1600},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Orfest","rank":2,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1597},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"NALP","rank":3,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1593},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"forest","rank":4,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1590},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"rem","rank":5,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1580},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"izbyshev","rank":6,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1587},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Sergey.Bankevich","rank":7,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1583},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"gusakov","rank":8,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1573},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"LinesPrower","rank":9,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1576},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"alt","rank":10,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1566},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"RAD","rank":11,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1570},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"pperm","rank":12,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1563},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Sammarize","rank":13,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1560},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"pank.dm","rank":14,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1560},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Irkhin","rank":15,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1553},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"romanandreev","rank":16,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1546},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"removed1","rank":17,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1549},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"mmaxio","rank":18,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1539},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"IgorTPH","rank":19,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1536},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Saeed_Reza","rank":20,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1543},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Efgen","rank":21,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1532},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"RAVEman","rank":22,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1529},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"maos","rank":23,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1526},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"spartac","rank":24,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1522},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"ft.azadi","rank":25,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1519},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"alexander.musman","rank":26,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1515},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Tim","rank":27,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1512},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"PanZverski","rank":28,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1509},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"P___","rank":29,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1505},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Fefer_Ivan","rank":30,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1502},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Edvard","rank":31,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1498},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Vashegin.Roman","rank":32,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1495},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"DarthBeleg","rank":33,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1492},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"steiner","rank":33,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1492},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"NikitaD","rank":35,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1485},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Igor_Kudryashov","rank":36,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1468},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"S.Yesipenko","rank":36,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1481},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Azadeh","rank":38,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1478},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"hydrastuff","rank":39,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1475},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"iensen","rank":40,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1471},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"ArtemKadeev","rank":41,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1453},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"San_Sany4","rank":41,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1464},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Salat","rank":43,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1461},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"GDR","rank":44,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1450},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"OSt","rank":45,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1458},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Babushkin","rank":46,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1456},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Babanin_Ivan","rank":47,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1455},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Narg","rank":48,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1452},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"wistful23","rank":49,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1451},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"TRR","rank":50,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1449},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Jens","rank":51,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1447},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"ann_z","rank":52,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1446},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"misterku","rank":53,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1445},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Fdg","rank":54,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1444},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Marvel","rank":55,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1443},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"JaA","rank":56,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1442},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"Xorand","rank":57,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1441},{"contestId":1,"contestName":"Codeforces Beta Round #1","handle":"oskirych","rank":57,"ratingUpdateTimeSeconds":1266588000,"oldRating":1500,"newRating":1441}]}"#;
    let packet: CodeforcesPacket<Vec<RatingChange>> =
        serde_json::from_str(EXAMPLE_CONTEST).expect("JSON parsing error");
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