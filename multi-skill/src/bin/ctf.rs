use multi_skill::data_processing::{try_write_slice_to_file, CURRENT_YEAR};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::StatusCode;
use select::document::Document;
use select::node::Node;
use select::predicate::Name;

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

struct ArchiveRows<'a, I: Iterator<Item = Node<'a>>> {
    table: I,
}

impl<'a, I: Iterator<Item = Node<'a>>> Iterator for ArchiveRows<'a, I> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let url_cell = self.table.next()?;
        let missing_cell = self.table.nth(5).unwrap();
        if missing_cell.text().starts_with("missing") {
            // This contest has no scoreboard, so skip it
            self.next()
        } else {
            let node = url_cell.find(Name("a")).next().unwrap();
            let path = node.attr("href").unwrap();
            let id = path.strip_prefix("/event/").unwrap();
            Some(id.parse().unwrap())
        }
    }
}

/*
struct ScoreboardRows<'a, I: Iterator<Item = Node<'a>>> {
    table: I,
}

impl<'a, I: Iterator<Item = Node<'a>>> Iterator for ScoreboardRows<'a, I> {
    type Item = (usize, String);

    fn next(&mut self) -> Option<Self::Item> {
        let place: usize = self.table.nth(1)?.text().parse().unwrap();
        let handle = self.table.next().unwrap().text();
        self.table.nth(3);
        Some((place, handle))
    }
}

for (contest_url, title, time_string) in table {
    tracing::info!("{} {} {}", contest_url, title, time_string);
    let contest_req = client.get(contest_url);
    let contest_page = request(contest_req).expect("Failed HTTP status");
    let title2 = contest_page.find(Name("h2")).next().unwrap().text();
    assert_eq!(title, title2);
    let time2 = contest_page.find(Name("p")).next().unwrap().text();
    let time2 = time2.split("— ").nth(1).unwrap();
    let time2 = time2.split(" UTC").next().unwrap();
    assert_eq!(time_string, time2);
    let table = ScoreboardRows {
        table: year_page.find(Name("td")),
    };
}
*/

fn main() {
    tracing_subscriber::fmt::init();
    tracing::error!(
        "The `ctf` binary is deprecated because the CTF API provides no means of \
        obtaining results by event ID. Use data_processing::fetch_ctf_history() instead."
    );

    let client = Client::new();
    let contest_ids = (2005..=CURRENT_YEAR)
        .flat_map(|year| {
            let url = format!("https://ctftime.org/event/list/?year={}", year);
            let req = client.get(url);
            let page = request(req).expect("Failed HTTP status");
            let table = ArchiveRows {
                table: page.find(Name("td")),
            };
            let urls = table.collect::<Vec<_>>();
            urls.into_iter().rev()
        })
        .collect::<Vec<_>>();

    try_write_slice_to_file(&contest_ids, "../data/ctf_contest_ids.json");
}
