//use crate::TorrentStats;
//use crate::SCRAPE_URL;
use diesel::dsl::IntoBoxed;
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use lava_torrent::bencode::BencodeElem;
//use reqwest::blocking::ClientBuilder;
use std::num::NonZeroU32;
const N_ITER: NonZeroU32 = NonZeroU32::new(100000).unwrap();

#[macro_export]
macro_rules! maperr {
    ($expr:expr) => {
        $expr.map_err(|e| e.to_string())
    };
}

#[derive(Debug, Default)]
pub struct Query {
    pub simples: Vec<String>,
    pub negated_simples: Vec<String>,
    pub simples_or: Vec<Vec<String>>,
    pub negated_simples_or: Vec<Vec<String>>,
    pub quotes: Vec<String>,
    pub negated_quotes: Vec<String>,
    pub quotes_or: Vec<Vec<String>>,
    pub negated_quotes_or: Vec<Vec<String>>,
}

pub fn format_hash(hash: &str) -> String {
    let mut output = String::new();
    for (idx, char) in hash.chars().enumerate() {
        if idx % 2 == 0 {
            output.push('%');
        }
        output.push(char);
    }
    output
}

pub fn bencode_to_usize(value: &BencodeElem) -> Result<usize, String> {
    let BencodeElem::Integer(value) = value else {
        return Err("BencodeElem is not an Integer!".to_string());
    };
    Ok(*value as usize)
}

pub fn torrent_from_hash(
    conn: &mut diesel::SqliteConnection,
    hash: &String,
) -> Option<Vec<crate::models::Torrent>> {
    use crate::models::Torrent;
    use crate::schema::torrents::dsl;
    let matches = dsl::torrents
        .filter(dsl::info_hash.eq(hash))
        .select(Torrent::as_select())
        .load(conn)
        .ok();
    if matches.as_ref().is_some_and(|m| m.is_empty()) {
        None
    } else {
        matches
    }
}

/*pub fn torrent_stats_from_hash(hash: &str) -> Result<Vec<TorrentStats>, String> {
    let client = maperr!(ClientBuilder::new().build())?;
    let formatted_hash = format_hash(hash);
    let url = format!("{SCRAPE_URL}?info_hash={formatted_hash}");
    let rq = maperr!(client.get(url).send())?;
    if !rq.status().is_success() {
        return Err(format!(
            "request to scrape endpoint returned {}",
            rq.status()
        ));
    }
    let resp = maperr!(rq.bytes())?;
    let bencoded = maperr!(BencodeElem::from_bytes(resp))?;
    bencoded.first().ok_or("bencoded response is empty!")?;
    let BencodeElem::Dictionary(dict) = bencoded.first().unwrap() else {
        return Err("bencoded response isn't a dictionary!".to_string());
    };
    dict.get("files")
        .ok_or("bencoded dictionary has no 'files' key!")?;
    let BencodeElem::RawDictionary(files) = dict.get("files").unwrap() else {
        return Err("bencoded dictionary key \"files\" is not a rawdictionary!".to_string());
    };
    let mut torrents = Vec::new();
    for (hash, data) in files.iter() {
        let hash = hex::encode(hash);
        let BencodeElem::Dictionary(data) = data else {
            return Err("hash \"{hash}\"'s value is not a dictionary!".to_string());
        };
        let complete = data
            .get("complete")
            .ok_or("hash has no \"complete\" key!")?;
        let incomplete = data
            .get("incomplete")
            .ok_or("hash has no \"incomplete\" key!")?;
        let downloaded = data
            .get("downloaded")
            .ok_or("hash has no \"downloaded\" key!")?;
        let torrent = TorrentStats {
            info_hash: hash,
            seeders: bencode_to_usize(complete)?,
            leechers: bencode_to_usize(incomplete)?,
            downloads: bencode_to_usize(downloaded)?,
        };
        torrents.push(torrent);
    }
    Ok(torrents)
}*/

pub fn verify_password(password: &str, hash: &str, salt: &str) -> bool {
    use data_encoding::HEXUPPER;
    use ring::pbkdf2;
    pbkdf2::verify(
        pbkdf2::PBKDF2_HMAC_SHA512,
        N_ITER,
        HEXUPPER.decode(salt.as_bytes()).unwrap().as_slice(),
        password.as_bytes(),
        HEXUPPER.decode(hash.as_bytes()).unwrap().as_slice(),
    )
    .is_ok()
}

pub fn hash_password(password: &String) -> Result<(String, String), ring::error::Unspecified> {
    use data_encoding::HEXUPPER;
    use ring::rand::SecureRandom;
    use ring::{digest, pbkdf2, rand};

    const CREDENTIAL_LEN: usize = digest::SHA512_OUTPUT_LEN;
    let rng = rand::SystemRandom::new();

    let mut salt = [0u8; CREDENTIAL_LEN];
    rng.fill(&mut salt)?;

    let mut pbkdf2_hash = [0u8; CREDENTIAL_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA512,
        N_ITER,
        &salt,
        password.as_bytes(),
        &mut pbkdf2_hash,
    );

    let salt = HEXUPPER.encode(&salt);
    let hash = HEXUPPER.encode(&pbkdf2_hash);

    assert!(verify_password(password, &hash, &salt));
    assert!(!verify_password(&format!("{password}a"), &hash, &salt));

    Ok((hash, salt))
}

pub fn parse_query(query: &str) -> Query {
    let mut result = Query::default();
    let mut token = String::new();
    let mut in_or = Vec::new();
    let mut quoted = false;
    let mut negated = false;
    for (idx, char) in query.chars().enumerate() {
        let next = query.chars().nth(idx + 1);
        negated = negated || char == '-';
        if quoted && char == '"' && next.is_none_or(|c| c != '|') {
            println!("closing quote: {}", next.is_none_or(|c| c != '|'));
            if !in_or.is_empty() {
                in_or.push(token.clone());
                if negated {
                    result.negated_quotes_or.push(in_or.clone());
                } else {
                    result.quotes_or.push(in_or.clone());
                }
                in_or = Vec::new();
            } else if in_or.is_empty() && negated {
                result.negated_quotes.push(token.clone());
            } else if in_or.is_empty() && !negated {
                result.quotes.push(token.clone());
            }
            token = String::new();
            quoted = false;
            negated = false;
            continue;
        }
        if quoted && char == '"' && next.is_some_and(|c| c == '|') {
            in_or.push(token.clone());
            token = String::new();
            quoted = false;
            continue;
        }
        if !quoted && char == '|' && !token.is_empty() {
            let malformed = next.is_none_or(|c| c == ' ');
            in_or.push(token.clone());
            if malformed && in_or.len() > 1 {
                if negated {
                    result.negated_simples_or.push(in_or.clone());
                } else {
                    result.simples_or.push(in_or.clone());
                }
                in_or = Vec::new();
            } else if malformed && in_or.len() == 1 {
                if negated {
                    result.negated_simples.push(in_or[0].clone());
                } else {
                    result.simples.push(in_or[0].clone());
                }
                in_or = Vec::new();
            }
            token = String::new();
        }
        if char == '|' && in_or.is_empty() {
            eprintln!("current char is pipe, but in_or is empty!");
        }
        quoted = quoted || char == '"';
        if (char != '-' && char != '"' && char != '|' && char != ' ')
            || (char == ' ' && quoted)
            || (char == '|' && quoted)
        {
            token.push(char);
        }
        if (char == ' ' || next.is_none()) && !quoted && !in_or.is_empty() {
            in_or.push(token.clone());
            if negated {
                result.negated_simples_or.push(in_or.clone());
            } else {
                result.simples_or.push(in_or.clone());
            }
            in_or = Vec::new();
            token = String::new();
            negated = false;
            continue;
        }
        if (char == ' ' || next.is_none()) && !quoted && !token.is_empty() {
            if negated {
                result.negated_simples.push(token.clone());
            } else {
                result.simples.push(token.clone());
            }
            token = String::new();
            negated = false;
            continue;
        }
    }
    result
}

pub fn escape_query(query: &str) -> String {
    let mut escaped = query.replace('\\', "\\\\");
    let chars = ['%', '_', '^', '[', ']', '-'];
    for char in chars {
        escaped = escaped.replace(char, &format!("\\{char}"));
    }
    escaped
}

pub fn search_torrent<'a>(
    query: &str,
) -> IntoBoxed<'a, crate::schema::torrents::dsl::torrents, Sqlite> {
    use crate::schema::torrents::{dsl, table};
    use diesel::sql_types::Bool;
    use diesel::sqlite::Sqlite;
    let query = parse_query(query);
    let mut matches = dsl::torrents.into_boxed();
    for quote in query.quotes {
        let query = format!("%{}%", escape_query(&quote));
        matches = matches.filter(dsl::title.like(query).escape('\\'));
    }
    for quote in query.negated_quotes {
        let query = format!("%{}%", escape_query(&quote));
        matches = matches.filter(dsl::title.not_like(query).escape('\\'));
    }
    for simple in query.simples {
        let query = format!("%{}%", escape_query(&simple));
        matches = matches.filter(dsl::title.like(query).escape('\\'));
    }
    for simple in query.negated_simples {
        let query = format!("%{}%", escape_query(&simple));
        matches = matches.filter(dsl::title.not_like(query).escape('\\'));
    }
    for group in query.simples_or {
        let query = format!("%{}%", escape_query(&group[0]));
        let mut condition: Box<dyn BoxableExpression<table, Sqlite, SqlType = Bool>> =
            Box::new(dsl::title.like(query).escape('\\'));
        for (idx, simple) in group.iter().enumerate() {
            if idx == 0 {
                continue;
            }
            let query = format!("%{}%", escape_query(simple));
            condition = Box::new(condition.or(dsl::title.like(query).escape('\\')));
        }
        matches = matches.filter(condition);
    }
    for group in query.negated_simples_or {
        let query = format!("%{}%", escape_query(&group[0]));
        let mut condition: Box<dyn BoxableExpression<table, Sqlite, SqlType = Bool>> =
            Box::new(dsl::title.not_like(query).escape('\\'));
        for (idx, simple) in group.iter().enumerate() {
            if idx == 0 {
                continue;
            }
            let query = format!("%{}%", escape_query(simple));
            condition = Box::new(condition.or(dsl::title.not_like(query).escape('\\')));
        }
        matches = matches.filter(condition);
    }
    for group in query.quotes_or {
        let query = format!("%{}%", escape_query(&group[0]));
        let mut condition: Box<dyn BoxableExpression<table, Sqlite, SqlType = Bool>> =
            Box::new(dsl::title.like(query).escape('\\'));
        for (idx, quote) in group.iter().enumerate() {
            if idx == 0 {
                continue;
            }
            let query = format!("%{}%", escape_query(quote));
            condition = Box::new(condition.or(dsl::title.like(query).escape('\\')));
        }
        matches = matches.filter(condition);
    }
    for group in query.negated_quotes_or {
        let query = format!("%{}%", escape_query(&group[0]));
        let mut condition: Box<dyn BoxableExpression<table, Sqlite, SqlType = Bool>> =
            Box::new(dsl::title.not_like(query).escape('\\'));
        for (idx, quote) in group.iter().enumerate() {
            if idx == 0 {
                continue;
            }
            let query = format!("%{}%", escape_query(quote));
            condition = Box::new(condition.or(dsl::title.not_like(query).escape('\\')));
        }
        matches = matches.filter(condition);
    }
    matches
}
