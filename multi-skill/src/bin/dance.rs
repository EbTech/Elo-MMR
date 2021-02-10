use reqwest::blocking::{Client, RequestBuilder};
use reqwest::StatusCode;
use select::document::Document;
use select::predicate::{Attr, Name};
use serde::Serialize;
use std::ops::RangeInclusive;

const ROOT_URL: &str = "https://results.o2cm.com/";

#[derive(Serialize)]
struct O2cmDateFilter {
    inyear: usize,
    inmonth: usize,
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize)]
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
        .expect("HTTP error: is the source website down?");
    if response.status().is_success() {
        let page_text = response
            .text()
            .expect("Failed to extract and decode the HTTP response body");
        Ok(Document::from(page_text.as_str()))
    } else {
        Err(response.status())
    }
}

fn get_urls(page: &Document) -> impl Iterator<Item = String> + '_ {
    page.select(Name("a")).map(|node| {
        let path = node.attr("href").expect("Missing href");
        format!("{}{}", ROOT_URL, path)
    })
}

fn get_range(page: &Document, property_name: &str) -> RangeInclusive<usize> {
    let node = page
        .select(Attr("name", property_name))
        .next()
        .unwrap_or_else(|| panic!("Can't find node with name={}", property_name));
    let min: usize = node
        .attr("min")
        .expect("Node has no 'min' attribute")
        .parse()
        .expect("Failed to parse 'min' attribute as integer");
    let max: usize = node
        .attr("max")
        .expect("Node has no 'max' attribute")
        .parse()
        .expect("Failed to parse 'max' attribute as integer");
    min..=max
}

fn process_heat(client: &Client, heat_page: Document) {
    println!("EXTRACTED {:?}", heat_page);
    /* TODO:
        Each table on a heat's page, EXCEPT for the "Summary" table (when it exists), belongs
        to a different round, and should be made into its own Contest object. You'll have to
        toggle the drop-down menu to access all the tables.

        In each contest, participants are ranked by aggregating all judges' verdicts.

        We'll have to think about whether to model couples as teams, and how to make
        the noise zero between rounds of the same comp without messing up the sigmas.
    */
}

fn main() {
    let client = Client::new();
    let root_req = client.get(ROOT_URL);
    let root_page = request(root_req).expect("Failed HTTP status");
    let year_range = get_range(&root_page, "inyear");
    let month_range = get_range(&root_page, "inmonth");
    let event_filter = O2cmEventFilter::default();
    let mut num_heats = 0;

    // The first 2 years contain no data, so we save time by skipping them
    for inyear in year_range.skip(2) {
        for inmonth in month_range.clone() {
            let date_filter = O2cmDateFilter { inyear, inmonth };
            let month_req = client.post(ROOT_URL).form(&date_filter);
            let month_page = request(month_req).expect("Failed HTTP status");

            // TODO: reverse iteration order to make it chronological
            for comp_url in get_urls(&month_page) {
                let comp_req = client.post(&comp_url).form(&event_filter);
                match request(comp_req) {
                    Ok(comp_page) => {
                        println!("{:2}/{} Processing {}", inmonth, inyear, comp_url);
                        for heat_url in get_urls(&comp_page).skip(1) {
                            //let heat_page = request(client.get(&heat_url)).expect("Failed status");
                            //process_heat(&client, heat_page);
                            num_heats += 1;
                        }
                        println!("Success! Processed {} heats so far.", num_heats);
                    }
                    Err(status) => {
                        eprintln!(
                            "{:2}/{} WARNING missing data: {} at {}",
                            inmonth, inyear, status, comp_url,
                        );
                    }
                }
            }
        }
    }
}
