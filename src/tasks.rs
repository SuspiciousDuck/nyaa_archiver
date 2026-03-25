use chrono::{TimeDelta, Utc};
use cron::Schedule;
use diesel::{
    BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper,
    SqliteConnection,
};
use std::{
    collections::VecDeque,
    fs::File,
    io::Write,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};
use tracker_lib::{
    establish_connection, mark_torrent_deleted,
    models::{DatabaseError, Item, Torrent},
    schema::torrents,
    scrape::{Client, ScrapeError},
};

fn main() -> Result<(), ScrapeError> {
    let client = Arc::new(Mutex::new(Client::new()?));
    let backlog_path = PathBuf::from("./backlog.json");
    if !backlog_path.try_exists().is_ok_and(|b| b) {
        let mut file = File::create(&backlog_path).unwrap();
        file.write_all(b"[]").unwrap();
    }
    let file = File::open(&backlog_path).unwrap();
    let backlog: Arc<Mutex<VecDeque<usize>>> =
        Arc::new(Mutex::new(serde_json::from_reader(&file).unwrap()));

    std::thread::scope(|s| {
        {
            let client = client.clone();
            let backlog = backlog.clone();
            s.spawn(move || {
                let connection = &mut establish_connection();
                // scrape latest page
                let expression = "0 */15 * * * *";
                let schedule = Schedule::from_str(expression).unwrap();

                loop {
                    let now = Utc::now();
                    // something is very wrong if this fails
                    let datetime = schedule.upcoming(Utc).take(1).next().unwrap();
                    let until = datetime - now;
                    std::thread::sleep(until.to_std().unwrap());
                    recursive_scrape_page(connection, client.clone(), backlog.clone(), 1, None);
                }
            });
        }
        {
            let client = client.clone();
            let backlog = backlog.clone();
            s.spawn(move || {
                let connection = &mut establish_connection();
                // deep scrape torrent backlog
                let expression = "10 */30 * * * *"; // happens 10 seconds after adding scheduled
                                                    // torrents so theres no race condition
                                                    // (probably)
                let schedule = Schedule::from_str(expression).unwrap();

                loop {
                    let now = Utc::now();
                    let datetime = schedule.upcoming(Utc).take(1).next().unwrap();
                    let until = datetime - now;
                    std::thread::sleep(until.to_std().unwrap());
                    while let Some(id) = {
                        let mut backlog = backlog.lock().unwrap();
                        let front = backlog.pop_front();
                        drop(backlog);
                        front
                    } {
                        let mut client = client.lock().unwrap();
                        let user_delta = TimeDelta::weeks(3);
                        let result = client.scrape_torrent(connection, id, &user_delta);
                        drop(client);
                        if let Err(err) = result {
                            eprintln!("{err}");
                            match err {
                                ScrapeError::TorrentDeleted(_) => {
                                    if let Err(e) = mark_torrent_deleted(connection, &id) {
                                        eprintln!("{e}");
                                    }
                                }
                                _ => {
                                    let mut backlog = backlog.lock().unwrap();
                                    backlog.push_back(id);
                                    drop(backlog);
                                }
                            }
                        }
                    }

                    let file = File::create(&backlog_path).unwrap();
                    serde_json::to_writer(&file, &*backlog).unwrap();
                }
            });
        }
        {
            let backlog = backlog.clone();
            s.spawn(move || {
                let connection = &mut establish_connection();
                // add torrents scheduled to rescan to backlog
                let expression = "0 */30 * * * *";
                let schedule = Schedule::from_str(expression).unwrap();

                loop {
                    let now = Utc::now();
                    let datetime = schedule.upcoming(Utc).take(1).next().unwrap();
                    let until = datetime - now;
                    std::thread::sleep(until.to_std().unwrap());
                    let date_threshold = (now - chrono::Duration::minutes(10)).timestamp() as i32;
                    let update_threshold = (now - chrono::Duration::weeks(1)).timestamp() as i32;
                    let torrents = torrents::table
                        .filter(torrents::deleted.eq(false))
                        .filter(
                            torrents::last_updated
                                .is_null()
                                .and(torrents::date.lt(date_threshold))
                                .or(torrents::last_updated.lt(update_threshold)),
                        )
                        .order(torrents::date.asc())
                        .limit(100)
                        .select(Torrent::as_select())
                        .load(connection)
                        .map_err(|e| DatabaseError::Search(Item::Torrent, e));
                    match torrents {
                        Ok(torrents) => {
                            let mut backlog = backlog.lock().unwrap();
                            for torrent in &torrents {
                                let date = chrono::DateTime::from_timestamp(torrent.date as i64, 0)
                                    .unwrap();
                                let date = if let Some(last_updated) = torrent.last_updated {
                                    let last =
                                        chrono::DateTime::from_timestamp(last_updated as i64, 0)
                                            .unwrap();
                                    date + TimeDelta::weeks((last - date).num_weeks())
                                } else {
                                    date
                                };
                                // by filtering anniversary-style, we avoid having constant
                                // massive backlogs due to updating torrents continuously
                                if (now - date).num_weeks() < 1 {
                                    continue;
                                }
                                backlog.push_back(torrent.id as usize);
                            }
                            drop(backlog);
                            println!("Added {} torrents to the backlog", torrents.len());
                        }
                        Err(e) => eprintln!("{e}"),
                    }
                }
            });
        }
    });
    Ok(())
}

fn recursive_scrape_page(
    connection: &mut SqliteConnection,
    client: Arc<Mutex<Client>>,
    backlog: Arc<Mutex<VecDeque<usize>>>,
    page: usize,
    latest: Option<usize>,
) {
    let latest = match latest {
        Some(latest) => Some(latest),
        None => get_latest_torrent(connection)
            .map(|l| l.first().copied())
            .inspect_err(|e| eprintln!("{e}"))
            .unwrap_or(None),
    };
    let result = {
        let mut client = client.lock().unwrap();
        let delta = TimeDelta::weeks(1);
        let user_delta = TimeDelta::weeks(3);
        let result = client.scrape_page(connection, page, "", true, &delta, &user_delta);
        drop(client);
        result
    };
    match result {
        Ok(torrents) => {
            if !torrents.1.is_empty() {
                let mut backlog = backlog.lock().unwrap();
                for torrent in &torrents.1 {
                    backlog.push_back(*torrent);
                }
                drop(backlog);
                println!("Added {} failed torrents to backlog", torrents.1.len());
            }
            match latest {
                Some(latest) => {
                    if torrents
                        .0
                        .last()
                        .is_some_and(|torrent| torrent.date > latest)
                    {
                        recursive_scrape_page(connection, client, backlog, page + 1, Some(latest));
                    }
                }
                None => eprintln!("Variable 'latest' is None!"),
            }
        }
        Err(err) => eprintln!("{err}"),
    }
}

fn get_latest_torrent(connection: &mut SqliteConnection) -> Result<Vec<usize>, DatabaseError> {
    torrents::table
        .order(torrents::date.desc())
        .limit(1)
        .select(Torrent::as_select())
        .load(connection)
        .map(|v| v.iter().map(|t| t.date as usize).collect())
        .map_err(|e| DatabaseError::Search(Item::Torrent, e))
}
