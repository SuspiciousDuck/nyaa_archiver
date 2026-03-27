use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

pub mod models;
pub mod schema;
pub mod scrape;
pub mod util;

use models::DatabaseError;
use models::Item;

//pub const SCRAPE_URL: &str = "http://192.168.0.5:10999/scrape";

/*#[derive(Debug)]
pub struct TorrentStats {
    pub info_hash: String,
    pub seeders: usize,
    pub leechers: usize,
    pub downloads: usize,
}*/

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set!");
    let mut conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {database_url}!"));
    conn.batch_execute("PRAGMA busy_timeout = 1000;")
        .expect("Failed to set busy_timeout!");
    conn
}

pub fn user_exists(conn: &mut SqliteConnection, username: &str) -> bool {
    use diesel::dsl::{exists, select};
    use schema::users::dsl;
    let is_present = select(exists(dsl::users.filter(dsl::username.eq(username)))).get_result(conn);
    is_present.is_ok_and(|b| b)
}

pub fn get_user(conn: &mut SqliteConnection, username: &str) -> Option<models::User> {
    use schema::users::dsl;
    dsl::users.find(username).first(conn).ok()
}

pub fn delete_user(
    conn: &mut SqliteConnection,
    username: &String,
) -> Result<Vec<models::User>, DatabaseError> {
    if !user_exists(conn, username) {
        return Err(DatabaseError::NotExists(Item::User, username.clone()));
    }

    use models::User;
    use schema::users::{self, dsl};

    diesel::delete(users::table)
        .filter(dsl::username.eq(username))
        .returning(User::as_returning())
        .load(conn)
        .map_err(|e| DatabaseError::Delete(Item::User, username.clone(), e))
}

pub fn get_torrent(conn: &mut SqliteConnection, id: usize) -> Option<models::Torrent> {
    use schema::torrents::dsl;
    let id = id as i32;
    dsl::torrents.find(id).first(conn).ok()
}

pub fn torrent_exists(conn: &mut SqliteConnection, id: usize) -> bool {
    use diesel::dsl::{exists, select};
    use schema::torrents;
    let id = id as i32;
    let is_present = select(exists(torrents::table.find(id))).get_result(conn);
    is_present.is_ok_and(|b| b)
}

pub fn torrent_deleted(conn: &mut SqliteConnection, id: usize) -> bool {
    use diesel::dsl::{exists, select};
    use schema::deleted_torrents;
    let deleted = select(exists(deleted_torrents::table.find(id as i32))).get_result(conn);
    deleted.is_ok_and(|b| b) || get_torrent(conn, id).is_some_and(|t| t.deleted)
}

pub fn mark_torrent_deleted(conn: &mut SqliteConnection, id: usize) -> Result<(), DatabaseError> {
    use models::DeletedTorrent;
    use schema::torrents;
    match get_torrent(conn, id) {
        None => Ok(()),
        Some(_) => diesel::update(torrents::table.find(id as i32))
            .set(torrents::deleted.eq(true))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::Update(Item::Torrent, id.to_string(), e)),
    }
    .and(DeletedTorrent { id: id as i32 }.insert(conn))
}

pub fn delete_torrent(
    conn: &mut SqliteConnection,
    id: usize,
) -> Result<Vec<models::Torrent>, DatabaseError> {
    if !torrent_exists(conn, id) {
        return Err(DatabaseError::NotExists(Item::Torrent, id.to_string()));
    }
    use models::Torrent;
    use schema::torrents::{self, dsl};
    let id = id as i32;

    diesel::delete(torrents::table)
        .filter(dsl::id.eq(id))
        .returning(Torrent::as_returning())
        .load(conn)
        .map_err(|e| DatabaseError::Delete(Item::Torrent, id.to_string(), e))
}

pub fn get_torrent_comments(conn: &mut SqliteConnection, id: usize) -> Vec<models::Comment> {
    use schema::comments::dsl;
    let id = id as i32;
    dsl::comments
        .filter(dsl::torrent_id.eq(id))
        .select(models::Comment::as_select())
        .load(conn)
        .unwrap_or_default()
}

pub fn get_comment(conn: &mut SqliteConnection, id: usize) -> Option<models::Comment> {
    use schema::comments::dsl;
    let id = id as i32;
    dsl::comments.find(id).first(conn).ok()
}

pub fn comment_exists(conn: &mut SqliteConnection, id: usize) -> bool {
    use diesel::dsl::{exists, select};
    use schema::comments::dsl;
    let id = id as i32;
    let is_present = select(exists(dsl::comments.filter(dsl::id.eq(id)))).get_result(conn);
    is_present.is_ok_and(|b| b)
}

pub fn delete_comment(
    conn: &mut SqliteConnection,
    id: usize,
) -> Result<Vec<models::Comment>, DatabaseError> {
    if !comment_exists(conn, id) {
        return Err(DatabaseError::NotExists(Item::Comment, id.to_string()));
    }
    use models::Comment;
    use schema::comments::{self, dsl};
    use schema::torrents::dsl as tor_dsl;
    let torrent_id = get_comment(conn, id).unwrap().torrent_id;
    let id = id as i32;

    let result = diesel::delete(comments::table)
        .filter(dsl::id.eq(id))
        .returning(Comment::as_returning())
        .load(conn)
        .map_err(|e| DatabaseError::Delete(Item::Comment, id.to_string(), e))?;
    diesel::update(tor_dsl::torrents)
        .filter(tor_dsl::id.eq(torrent_id))
        .set(tor_dsl::comments.eq(tor_dsl::comments - 1))
        .execute(conn)
        .map_err(|e| DatabaseError::Update(Item::Torrent, torrent_id.to_string(), e))?;
    Ok(result)
}
