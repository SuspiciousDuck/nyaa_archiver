## nyaa_archiver
archive as much as possible from nyaa.si and save it on a local sqlite database. web frontend to view database. uses tor to anonymously fetch data.

### setup
```bash
$ git clone --depth 1 --single-branch https://github.com/SuspiciousDuck/nyaa_archiver
$ cd nyaa_archiver
$ mkdir torrents pfps
$ cargo install diesel_cli --no-default-features --features sqlite
$ diesel setup
$ cargo update
$ cargo run --release --bin scraper # to archive a specific id or page
$ cargo run --release --bin tasks # automatically scrape nyaa.si
$ cargo run --release --bin api # host web frontend to view your archive locally at http://localhost:11000/
```