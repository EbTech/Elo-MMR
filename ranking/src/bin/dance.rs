use reqwest::blocking::{Client, RequestBuilder};
use reqwest::StatusCode;
use select::document::Document;
use select::predicate::{Attr, Name};
use serde::Serialize;
use std::ops::RangeInclusive;

#[derive(Serialize)]
struct O2cmDateFilter {
    inyear: usize,
    inmonth: usize,
}

fn request(builder: RequestBuilder) -> Result<Document, StatusCode> {
    let response = builder
        .send()
        .expect("HTTP error: is results.o2cm.com down?");
    if response.status().is_success() {
        let page_text = response
            .text()
            .expect("Failed to extract and decode the HTTP response body");
        Ok(Document::from(page_text.as_str()))
    } else {
        Err(response.status())
    }
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

fn process_comp(comp_page: Document) {
    println!("EXTRACTED {:?}", comp_page);
    /* TODO:
        - for each competition of this month,
        - for each [choose filter] in this competition,
        - for each category in the [choose filter],
        - for each heat in this category,
        - make a Contest object, competitors ranked by aggregate of all judges

      The [choose filter] step is needed to make the list of heats small enough to
      display on one page. Note that any table labeled "Summary" must be ignored,
      but any extra tables hidden under a round drop-down must be included.

      We'll have to think about whether to model couples as teams,
      and how to add zero noise within a comp without messing up the sigmas.
    */
}

fn main() {
    const ROOT_URL: &str = "https://results.o2cm.com/";

    let client = Client::new();
    let root_req = client.get(ROOT_URL);
    let root_page = request(root_req).expect("Failed status");
    let year_range = get_range(&root_page, "inyear");
    let month_range = get_range(&root_page, "inmonth");

    for inyear in year_range {
        for inmonth in month_range.clone() {
            let date_filter = O2cmDateFilter { inyear, inmonth };
            let month_req = client.post(ROOT_URL).form(&date_filter);
            let month_page = request(month_req).expect("Failed status");

            for comp_node in month_page.select(Name("a")) {
                let comp_path = comp_node.attr("href").expect("Missing href");
                let comp_url = format!("{}{}", ROOT_URL, comp_path);
                let comp_req = client.get(&comp_url);
                match request(comp_req) {
                    Ok(comp_page) => {
                        process_comp(comp_page);
                    }
                    Err(status) => {
                        eprintln!(
                            "WARNING: {} at {}  ~~  There might be no data on {} {}.",
                            status,
                            comp_url,
                            comp_node.text(),
                            inyear,
                        );
                        continue;
                    }
                }
            }
        }
    }
}
