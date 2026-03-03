use diesel::prelude::*;
use std::io::{Write, stdin, stdout};
use tracker_lib::models::Torrent;
use tracker_lib::schema::torrents::dsl;
use tracker_lib::util::torrent_stats_from_hash;
use tracker_lib::{establish_connection, get_torrent};

fn input(msg: &str) -> String {
    let mut resp = String::new();
    print!("{msg}");
    stdout().flush().unwrap();
    stdin().read_line(&mut resp).unwrap();
    resp.trim_end().to_string()
}

fn main() {
    let connection = &mut establish_connection();
    let id = input("id: ").parse::<usize>().unwrap();
    let torrent = get_torrent(connection, &id).expect("No torrents matched id!");
    let info_hash = torrent.info_hash;
    let torrent_stats = &torrent_stats_from_hash(&info_hash.unwrap())
        .expect("Failed to fetch torrent statistics!")[0];
    println!("Fetched torrent statistics: {torrent_stats:?}");

    macro_rules! set {
        ($expr:expr) => {
            diesel::update(dsl::torrents.find(id as i32))
                .set($expr)
                .returning(Torrent::as_returning())
                .get_result(connection)
                .unwrap()
        };
    }
    set!(dsl::seeders.eq(torrent_stats.seeders as i32));
    set!(dsl::leechers.eq(torrent_stats.leechers as i32));
    let torrent = set!(dsl::completed.eq(torrent_stats.downloads as i32));
    println!(
        "Updated torrent {}: seeders = {}, leechers = {}, downloads = {}",
        torrent.id, torrent.seeders, torrent.leechers, torrent.completed
    );
}
