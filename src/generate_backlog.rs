use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use std::fs::File;
use tracker_lib::{models::Torrent, schema::torrents, torrent_deleted, torrent_exists};

fn main() {
    let conn = &mut tracker_lib::establish_connection();
    let last_torrent = torrents::table
        .order(torrents::id.asc())
        .select(Torrent::as_select())
        .first(conn)
        .unwrap();
    let latest_torrent = torrents::table
        .order(torrents::id.desc())
        .select(Torrent::as_select())
        .first(conn)
        .unwrap();
    let mut torrents = Vec::new();
    for id in (last_torrent.id..=latest_torrent.id).rev() {
        let id = id as usize;
        if !torrent_exists(conn, &id) && !torrent_deleted(conn, &id) {
            torrents.push(id);
        }
    }
    let file = File::create("./backlog.json").unwrap();
    let serialize = serde_json::to_value(torrents).unwrap();
    serde_json::to_writer(file, &serialize).unwrap();
}
