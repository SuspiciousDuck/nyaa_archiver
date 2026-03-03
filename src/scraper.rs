use chrono::TimeDelta;
use std::{
    io::{stdin, stdout, Read, Write},
    sync::{Arc, Mutex},
};
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
    let delta = TimeDelta::days(1);
    let user_delta = TimeDelta::weeks(1);
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
            .scrape_page(connection, page, &options, deep, &delta, &user_delta)
            .map(|_| ())
    } else {
        let id = id.parse::<usize>().expect("Recieved non usize!");
        client.scrape_torrent(connection, id, &user_delta)
    }
}
