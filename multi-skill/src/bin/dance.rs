use chrono::prelude::*;
use chrono::{DateTime, Utc};
use multi_skill::data_processing::{write_to_json, Contest};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::StatusCode;
use select::document::Document;
use select::predicate::{And, Attr, Class, Name, Or};
use serde::Serialize;
use std::collections::HashSet;
use std::ops::RangeInclusive;

const ROOT_URL: &str = "https://results.o2cm.com/";

#[derive(Serialize)]
struct O2cmDateFilter {
    inyear: u32,
    inmonth: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct O2cmEventFilter {
    sel_div: String,
    sel_age: String,
    sel_skl: String,
    sel_sty: String,
    sel_ent: String,
    submit: String,
}

/// The default filter is no filter at all, yielding all the dance heats
impl Default for O2cmEventFilter {
    fn default() -> Self {
        Self {
            sel_div: "".to_string(),
            sel_age: "".to_string(),
            sel_skl: "".to_string(),
            sel_sty: "".to_string(),
            sel_ent: "".to_string(),
            submit: "OK".to_string(),
        }
    }
}

fn request(builder: RequestBuilder) -> Result<Document, StatusCode> {
    let response = builder
        .send()
        .expect("HTTP error: is the source website down?")
        .error_for_status()
        .map_err(|e| e.status().unwrap())?;
    let page_text = response
        .text()
        .expect("Failed to extract and decode the HTTP response body");
    Ok(Document::from(page_text.as_str()))
}

fn get_range(page: &Document, property_name: &str) -> RangeInclusive<u32> {
    let node = page
        .select(Attr("name", property_name))
        .next()
        .unwrap_or_else(|| panic!("Can't find node with name={}", property_name));
    let min: u32 = node
        .attr("min")
        .expect("Node has no 'min' attribute")
        .parse()
        .expect("Failed to parse 'min' attribute as integer");
    let max: u32 = node
        .attr("max")
        .expect("Node has no 'max' attribute")
        .parse()
        .expect("Failed to parse 'max' attribute as integer");
    min..=max
}

fn get_dates_and_urls(page: &Document) -> Vec<(u32, String)> {
    let mut res: Vec<(u32, String)> = Vec::new();
    // Every second one is a date
    for row in page.select(And(Class("t1n"), Name("tr"))) {
        let children: Vec<_> = row.children().collect();
        assert_eq!(children.len(), 2);

        let date_text = children[0].text();
        let tokens: Vec<_> = date_text.split(' ').collect();
        let date: u32 = tokens[1].parse().unwrap();

        let link = children[1].first_child().expect("Missing contest link.");
        let path = link.attr("href").expect("Missing href");
        let url = format!("{}{}", ROOT_URL, path);

        res.push((date, url));
    }
    res.sort();
    res
}

fn get_rounds(page: &Document) -> impl Iterator<Item = String> + '_ {
    let pred = Or(
        And(Class("t2b"), Name("td")),
        Or(Name("a"), And(Class("t2n"), Name("td"))),
    );
    page.select(pred).skip(5).map(|node| {
        if node.name() == Some("a") {
            // Delimit round names with special characters
            "$ ".to_string() + &node.text()
        } else {
            node.text()
        }
    })
}

fn process_round(mut round: Vec<(usize, String)>) -> Vec<(String, usize, usize)> {
    let mut lo = 0;
    let mut hi = 0;
    let mut res: Vec<(String, usize, usize)> = vec![];
    while lo < round.len() {
        let cur = round[hi].0;
        while hi + 1 < round.len() && round[hi + 1].0 == cur {
            hi += 1;
        }
        for j in lo..=hi {
            let name = std::mem::take(&mut round[j].1);
            res.push((name, lo, hi));
        }
        lo = hi + 1;
        hi = lo;
    }
    res
}

fn write_round(
    round: Vec<(usize, String)>,
    contest_name: &String,
    round_name: &String,
    num_rounds: &mut usize,
    datetime: DateTime<Utc>,
) {
    if round.len() > 1 {
        let contest = Contest {
            name: format!("{} {}", contest_name, round_name),
            url: None, // TODO: add scoresheet URL
            weight: 1.0,
            time_seconds: datetime.timestamp() as u64,
            standings: process_round(round),
        };
        std::fs::create_dir_all("../cache/dance").expect("Could not create cache directory");
        let path = format!("../cache/dance/{}.json", num_rounds);
        let write_res = write_to_json(&contest, &path);
        match write_res {
            Ok(()) => tracing::info!("Successfully wrote to {:?}", path),
            Err(msg) => tracing::error!("WARNING: failed write to {:?} because {}", path, msg),
        };
        *num_rounds += 1;
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let client = Client::new();
    let root_req = client.get(ROOT_URL);
    let root_page = request(root_req).expect("Failed HTTP status");
    let year_range = get_range(&root_page, "inyear");
    let month_range = get_range(&root_page, "inmonth");
    let event_filter = O2cmEventFilter::default();
    let mut num_rounds = 0;

    // The first 2 years contain no data, so we save time by skipping them
    for inyear in year_range.skip(2) {
        for inmonth in month_range.clone() {
            let date_filter = O2cmDateFilter { inyear, inmonth };
            let month_req = client.post(ROOT_URL).form(&date_filter);
            let month_page = request(month_req).expect("Failed HTTP status");

            for (inday, comp_url) in get_dates_and_urls(&month_page) {
                let comp_req = client.post(&comp_url).form(&event_filter);
                match request(comp_req) {
                    Ok(comp_page) => {
                        tracing::info!(
                            "{:2}/{:2}/{} Processing {}",
                            inday,
                            inmonth,
                            inyear,
                            comp_url
                        );

                        let contest_name = match comp_page.select(Class("h4")).next() {
                            Some(node) => node.text(),
                            None => "Nameless Contest".to_string(),
                        };
                        let mut round: Vec<(usize, String)> = vec![];
                        let mut round_name = "".to_string();
                        let mut names = HashSet::new();
                        for line in get_rounds(&comp_page) {
                            // Split string into tokens and get the placing and name
                            let tokens: Vec<_> = line.split(' ').collect();
                            if tokens[0] == "$" {
                                if !round.is_empty() {
                                    write_round(
                                        std::mem::take(&mut round),
                                        &contest_name,
                                        &round_name,
                                        &mut num_rounds,
                                        /*This can be scraped from the O2CM results search. We'll leave it for now*/
                                        Utc.ymd(inyear as i32, inmonth, inday).and_hms(0, 0, 0),
                                    );
                                }
                                round_name = tokens[1..tokens.len()].join(" ").to_string();
                            } else {
                                let team = if tokens.contains(&"-") {
                                    tokens[2..tokens.len() - 2].join(" ").to_string()
                                } else {
                                    tokens[2..tokens.len()].join(" ").to_string()
                                };
    
                                // Remove any forward slashes
                                let sanitized_team = team.replace("/", "");

                                // Remove dups
                                if names.contains(&sanitized_team) {
                                    continue;
                                }
                                names.insert(sanitized_team.clone());

                                // Remove any forward slashes
                                let sanitized_team = team.replace("/", "");

                                // Remove dups
                                if names.contains(&sanitized_team) {
                                    continue;
                                }
                                names.insert(sanitized_team.clone());

                                // Check if new round by seeing if this is a first place
                                let rank: usize = tokens[0][..tokens[0].len() - 1].parse().unwrap();
                                round.push((rank, sanitized_team));
                            }
                        }
                        write_round(
                            round,
                            &contest_name,
                            &round_name,
                            &mut num_rounds,
                            Utc.ymd(inyear as i32, inmonth, inday).and_hms(0, 0, 0),
                        );
                    }
                    Err(status) => {
                        tracing::warn!(
                            "{:2}/{:2}/{} missing data: {} at {}",
                            inday,
                            inmonth,
                            inyear,
                            status,
                            comp_url,
                        );
                    }
                }
            }
        }
    }
}
