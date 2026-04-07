use std::path::PathBuf;

use diesel::connection::DefaultLoadingMode;
use diesel::prelude::*;

use chrono::{prelude::*, TimeDelta};

use lava_torrent::torrent::v1::Torrent as LavaTorrent;

use tracker_lib::models::Torrent;
use tracker_lib::schema;
use tracker_lib::{establish_connection, get_torrent_comments};

fn main() {
    let conn = &mut establish_connection();
    let conn_2 = &mut establish_connection();
    let mut remove_information = Vec::new();
    let mut remove_description = Vec::new();
    let mut insert_scan_schedule = Vec::new();
    let torrents = schema::torrents::table
        .load_iter::<Torrent, DefaultLoadingMode>(conn)
        .unwrap()
        .filter_map(Result::ok);
    println!("Loaded all torrents!");

    for torrent in torrents {
        let torrent_path = PathBuf::from(format!("./torrents/{}.torrent", torrent.id));
        if !torrent_path.try_exists().is_ok_and(|e| e) {
            eprintln!(
                "The torrent {} does not have an actual torrent downloaded!",
                torrent.id
            );
        } else {
            let torrent_file = LavaTorrent::read_from_file(torrent_path).unwrap();

            if torrent_file.length != torrent.size {
                eprintln!("The torrent {} has a mismatching size field!", torrent.id);
            }

            if torrent.info_hash != torrent_file.info_hash() {
                eprintln!(
                    "The torrent {} has a mismatched infohash field!",
                    torrent.id
                );
            }
        }

        if torrent.submitter.is_none() && !torrent.anonymous {
            eprintln!(
                "The torrent {} isn't anonymous but the submitter is None!",
                torrent.id
            );
        }

        if torrent
            .information
            .is_some_and(|info| info == "No information.")
        {
            eprintln!(
                "The torrent {}'s information field is Some when it should be None!",
                torrent.id
            );
            remove_information.push(torrent.id);
        }

        if torrent
            .description
            .is_some_and(|desc| desc == "#### No description.")
        {
            eprintln!(
                "The torrent {}'s description field is Some when it should be None!",
                torrent.id
            );
            remove_description.push(torrent.id);
        }

        if torrent.comments as usize != get_torrent_comments(conn_2, torrent.id as usize).len() {
            eprintln!(
                "The torrent {}'s comments field is not equal to the number of comments!",
                torrent.id
            );
        }

        if torrent.next_update.as_ref().is_none() && torrent.update_frequency.as_ref().is_none() {
            let now = Utc::now();
            let delta = now
                .signed_duration_since(DateTime::from_timestamp_secs(torrent.date).unwrap())
                .min(TimeDelta::days(365))
                .max(TimeDelta::hours(1));
            let next_update = (now + delta).timestamp();

            insert_scan_schedule.push((torrent.id, next_update, delta.num_minutes() as i32));
        }
    }

    for id in remove_information {
        diesel::update(schema::torrents::table.find(id))
            .set(schema::torrents::information.eq(None::<String>))
            .execute(conn)
            .unwrap();
    }

    for id in remove_description {
        diesel::update(schema::torrents::table.find(id))
            .set(schema::torrents::description.eq(None::<String>))
            .execute(conn)
            .unwrap();
    }

    for (id, next_update, update_frequency) in insert_scan_schedule {
        diesel::update(schema::torrents::table.find(id))
            .set(schema::torrents::next_update.eq(Some(next_update)))
            .execute(conn)
            .unwrap();

        diesel::update(schema::torrents::table.find(id))
            .set(schema::torrents::update_frequency.eq(Some(update_frequency)))
            .execute(conn)
            .unwrap();
    }
}
