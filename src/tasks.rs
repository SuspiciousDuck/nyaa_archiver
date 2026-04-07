use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{prelude::*, TimeDelta};

use cron::Schedule;

use diesel::prelude::*;

use tracker_lib::models::DatabaseError;
use tracker_lib::scrape::{Client, ScrapeError};
use tracker_lib::{establish_connection, torrent_deleted, torrent_exists};
use tracker_lib::{mark_torrent_deleted, schema};

fn main() -> Result<(), ScrapeError> {
    let connection = Arc::new(Mutex::new(establish_connection()));
    let client = Arc::new(Mutex::new(Client::new()?));

    println!("Performing initial page scrapes...");
    client.lock().unwrap().scrape_page(
        &mut connection.lock().unwrap(),
        1,
        "",
        true,
        next_update_algo,
    )?;
    client.lock().unwrap().scrape_page(
        &mut connection.lock().unwrap(),
        1,
        "&o=asc",
        true,
        next_update_algo,
    )?;

    println!("Generating backlog list...");
    let backlog = Arc::new(Mutex::new(generate_backlog(
        &mut connection.lock().unwrap(),
    )?));
    println!("Initialized");

    std::thread::scope(|s| {
        {
            let connection = connection.clone();
            let client = client.clone();
            let backlog = backlog.clone();
            s.spawn(move || {
                let expression = "0 */15 * * * *";
                let schedule = Schedule::from_str(expression).unwrap();

                loop {
                    let now = Utc::now();
                    let datetime = schedule.upcoming(Utc).take(1).next().unwrap();
                    let until = datetime - now;
                    println!("Next page scrape in {} minutes", until.num_minutes());
                    std::thread::sleep(until.to_std().unwrap());
                    println!("Scraping latest page");

                    let mut connection = connection.lock().unwrap();
                    let mut client = client.lock().unwrap();
                    let result = client.scrape_page(&mut connection, 1, "", true, next_update_algo);

                    match result {
                        Ok((_, errors)) => errors.iter().for_each(|id| {
                            eprintln!("{}", ScrapeError::TorrentMissing(*id));
                            backlog.lock().unwrap().push_back(*id);
                        }),
                        Err(err) => eprintln!("{err}"),
                    }
                }
            });
        }
        {
            let connection = connection.clone();
            let client = client.clone();
            let backlog = backlog.clone();
            s.spawn(move || loop {
                let mut connection = connection.lock().unwrap();
                let mut client = client.lock().unwrap();
                let mut backlog = backlog.lock().unwrap();
                let now = Utc::now();

                use schema::torrents as table;
                let torrent = table::table
                    .order((table::update_count.asc(), table::date.asc()))
                    .filter(table::next_update.lt(now.timestamp()))
                    .filter(table::deleted.eq(false))
                    .select(table::id)
                    .first::<i32>(&mut *connection)
                    .inspect_err(|err| match err {
                        diesel::result::Error::NotFound => (),
                        _ => {
                            eprintln!("Encountered error while fetching torrent to process!: {err}")
                        }
                    })
                    .ok()
                    .or(backlog.pop_front().map(|id| id as i32));

                if let Some(id) = torrent {
                    let result =
                        client.scrape_torrent(&mut connection, id as usize, next_update_algo);

                    let _ = result.as_ref().inspect_err(|error| eprintln!("{error}"));
                    match result.err() {
                        Some(ScrapeError::TorrentDeleted(id)) => {
                            let _ = mark_torrent_deleted(&mut connection, id)
                                .inspect_err(|error| eprintln!("{error}"));
                        }
                        Some(ScrapeError::TorrentMissing(id)) => backlog.push_back(id),
                        _ => (),
                    };
                    drop(connection);
                    drop(client);
                    drop(backlog);
                } else {
                    drop(connection);
                    drop(client);
                    drop(backlog);
                    eprintln!("No torrents to be updated or in backlog.");
                    std::thread::sleep(Duration::from_mins(10));
                }
                std::thread::sleep(Duration::from_secs(1));
                // if we dont sleep 1 second, thread 1 will never get a lock
            });
        }
    });

    Ok(())
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

fn generate_backlog(connection: &mut SqliteConnection) -> Result<VecDeque<usize>, DatabaseError> {
    use schema::torrents as table;
    let oldest = table::table
        .order(table::id.asc())
        .select(table::id)
        .first::<i32>(connection)
        .map_err(DatabaseError::Generic)?;
    let newest = table::table
        .order(table::id.desc())
        .select(table::id)
        .first::<i32>(connection)
        .map_err(DatabaseError::Generic)?;

    Ok((oldest..=newest)
        .rev()
        .map(|id| id as usize)
        .filter(|id| !torrent_exists(connection, *id) && !torrent_deleted(connection, *id))
        .collect())
}
