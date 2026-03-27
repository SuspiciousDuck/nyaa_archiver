use std::io::{stdin, stdout, Read, Write};

use tracker_lib::{delete_torrent, establish_connection, models::DatabaseError};

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

fn main() -> Result<(), DatabaseError> {
    let connection = &mut establish_connection();
    let ids = input("ids (comma separated):", false);
    for id in ids.split(',') {
        let id = id.parse::<usize>().expect("Recieved non usize!");
        delete_torrent(connection, id)?;
    }

    Ok(())
}
