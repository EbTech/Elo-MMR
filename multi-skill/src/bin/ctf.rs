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
    type Item = (String, String, String);

    fn next(&mut self) -> Option<Self::Item> {
        let url_cell = self.table.next()?;
        let date_cell = self.table.next().unwrap();
        let missing_cell = self.table.nth(4).unwrap();
        if missing_cell.text().starts_with("missing") {
            // This contest has no scoreboard, so skip it
            self.next()
        } else {
            let node = url_cell.select(Name("a")).next().unwrap();
            let path = node.attr("href").unwrap();
            let url = format!("https://ctftime.org{}", path);
            let title = node.text();
            let time_string = date_cell.text().split(" — ").nth(1).unwrap().to_owned();
            Some((url, title, time_string))
        }
    }
}

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

fn process_heat(client: &Client, heat_page: Document) {
    tracing::info!("EXTRACTED {:?}", heat_page);
}

fn main() {
    tracing_subscriber::fmt::init();

    let client = Client::new();

    for year in 2005..=2021 {
        let year_url = format!("https://ctftime.org/event/list/?year={}", year);
        let year_req = client.get(year_url);
        let year_page = request(year_req).expect("Failed HTTP status");
        let table = ArchiveRows {
            table: year_page.select(Name("td")),
        };
        // TODO: reverse rows, parse time in seconds, parse Contest
        for (contest_url, title, time_string) in table {
            tracing::info!("{} {} {}", contest_url, title, time_string);
            let contest_req = client.get(contest_url);
            let contest_page = request(contest_req).expect("Failed HTTP status");
            let table = ScoreboardRows {
                table: year_page.select(Name("td")),
            };
        }
    }
}
