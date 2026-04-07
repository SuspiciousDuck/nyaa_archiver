use chrono::prelude::*;
use chrono::TimeDelta;
use std::io::{stdin, stdout, Read, Write};
use tracker_lib::{establish_connection, scrape::*};

fn input(msg: &str, eof: bool) -> String {
    let mut resp = String::new();
    print!("{msg}");
    stdout().flush().unwrap();
    if !eof {
        stdin().read_line(&mut resp).unwrap();
    } else {
        let mut buf = Vec::new();
        stdin().read_to_end(&mut buf).unwrap();
        resp = String::from_utf8(buf).unwrap();
    }
    resp.trim_end().to_string()
}

fn main() -> Result<(), ScrapeError> {
    let mut client = Client::new()?;
    let id = input("id: ", false);
    let connection = &mut establish_connection();
    if id.is_empty() {
        let page = input("page: ", false)
            .parse::<usize>()
            .expect("Recieved non usize!");
        let deep = input("deep: ", false);
        let deep = if deep.is_empty() {
            false
        } else {
            deep.parse::<bool>().expect("Recieved non bool!")
        };
        let options = input("options: ", false);
        client
            .scrape_page(connection, page, &options, deep, next_update_algo)
            .map(|_| ())
    } else {
        let id = id.parse::<usize>().expect("Recieved non usize!");
        client.scrape_torrent(connection, id, next_update_algo)
    }
}

fn next_update_algo(
    new: bool,
    differs: bool,
    date: i64,
    update_frequency: Option<i32>,
) -> (i64, i32) {
    let now = Utc::now();

    let delta = if new {
        now.signed_duration_since(DateTime::from_timestamp_secs(date).unwrap())
    } else if differs {
        TimeDelta::hours(1)
    } else {
        TimeDelta::minutes(update_frequency.unwrap_or(60) as i64) * 2
    }
    .min(TimeDelta::days(365));

    let delta = if delta.num_minutes() == 0 {
        TimeDelta::hours(1)
    } else {
        delta
    };

    let next_update = (now + delta).timestamp();

    (next_update, delta.num_minutes() as i32)
}
