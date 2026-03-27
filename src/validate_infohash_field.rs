use std::path::PathBuf;

use diesel::connection::DefaultLoadingMode;
use diesel::prelude::*;

use lava_torrent::torrent::v1::Torrent as LavaTorrent;

use tracker_lib::establish_connection;
use tracker_lib::models::Torrent;
use tracker_lib::schema;

fn main() {
    let conn = &mut establish_connection();
    let torrents = schema::torrents::table
        .load_iter::<Torrent, DefaultLoadingMode>(conn)
        .unwrap()
        .filter_map(Result::ok);
    println!("Loaded all torrents!");

    for torrent in torrents {
        let torrent_path = PathBuf::from(format!("./torrents/{}.torrent", torrent.id));

        if torrent_path.try_exists().is_ok_and(|e| e) {
            let torrent_file = LavaTorrent::read_from_file(torrent_path).unwrap();

            if torrent.info_hash != torrent_file.info_hash() {
                eprintln!(
                    "The torrent {} has a mismatched infohash field!",
                    torrent.id
                );
            }
        }
    }
}
