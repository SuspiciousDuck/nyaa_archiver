const tracker_url: &str = "http://localhost:10999";
const scrape_url: &str = "http://localhost:10999/scrape";

fn format_hash(hash: &str) -> String {
    let mut output = String::new();
    for (idx, char) in hash.chars().enumerate() {
        if idx % 2 == 0 {
            output.push('%');
        }
        output.push(char);
    }
    output
}

fn bencode_to_usize(value: &lava_torrent::bencode::BencodeElem) -> Result<usize, String> {
    let lava_torrent::bencode::BencodeElem::Integer(value) = value else {
        return Err("BencodeElem is not an Integer!".to_string());
    };
    Ok(*value as usize)
}

#[derive(Debug)]
struct TorrentStats {
    info_hash: String,
    seeders: usize,
    leechers: usize,
    downloads: usize,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let client = reqwest::blocking::ClientBuilder::new().build().unwrap();
    let hash = &args[1];
    let rq = client
        .get(format!("{scrape_url}?info_hash={}", format_hash(hash)))
        .send()
        .unwrap();
    let resp = rq.bytes().unwrap();
    let bencoded = lava_torrent::bencode::BencodeElem::from_bytes(resp).unwrap();
    let lava_torrent::bencode::BencodeElem::Dictionary(dict) = bencoded.first().unwrap() else {
        panic!("bencoded response is empty!")
    };
    let lava_torrent::bencode::BencodeElem::RawDictionary(files) = dict.get("files").unwrap()
    else {
        panic!("bencoded response has no \"files\" key!")
    };
    let mut torrents = Vec::new();
    for (hash, data) in files.iter() {
        let hash = hex::encode(hash);
        let lava_torrent::bencode::BencodeElem::Dictionary(data) = data else {
            panic!("hash \"{hash}\"'s value is not a dictionary!")
        };
        let torrent = TorrentStats {
            info_hash: hash,
            seeders: bencode_to_usize(data.get("complete").unwrap()).unwrap(),
            leechers: bencode_to_usize(data.get("incomplete").unwrap()).unwrap(),
            downloads: bencode_to_usize(data.get("downloaded").unwrap()).unwrap(),
        };
        torrents.push(torrent);
    }
    println!("{torrents:#?}");
    Ok(())
}
