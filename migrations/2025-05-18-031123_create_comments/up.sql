CREATE TABLE comments (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
  torrent_id INTEGER NOT NULL,
  submitter VARCHAR NOT NULL,
  date_created INTEGER NOT NULL,
  date_edited INTEGER,
  text VARCHAR NOT NULL,
  CONSTRAINT fk_torrents
    FOREIGN KEY (torrent_id)
    REFERENCES torrents(id)
    ON DELETE CASCADE
);
