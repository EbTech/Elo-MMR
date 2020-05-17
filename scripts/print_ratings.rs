// Copy-paste a spreadsheet column of CF handles as input to this program, then
// paste this program's output into the spreadsheet's ratings column.
use std::collections::HashMap;
use std::fs::File;
use std::io;
use io::{BufRead, Write};

fn main() {
    let mut ratings = HashMap::new();
    let file = File::open("../data/CFratings.txt").expect("File not found");
    let buf_file = io::BufReader::new(file);

    for line in buf_file.lines().map(|l| l.expect("Failed file read")) {
        if &line[11..12] == "(" {
            let rating = line[7..11].trim().to_owned();
            let handle = line[17..41].trim().to_owned();
            ratings.insert(handle, rating);
        }
    }

    let (stdin, stdout) = (io::stdin(), io::stdout());
    let mut out = io::BufWriter::new(stdout.lock());
    for handle in stdin.lock().lines().map(|l| l.expect("Failed stdin read")) {
	    if handle == "FLUSH" {
		    out.flush().ok();
	    }
        let rating = ratings.get(&handle).map(|r| r.as_str()).unwrap_or("");
        writeln!(out, "{}", rating).ok();
    }
}
