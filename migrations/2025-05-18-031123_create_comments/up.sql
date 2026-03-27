CREATE TABLE comments (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
  torrent_id INTEGER NOT NULL,
  submitter VARCHAR NOT NULL,
  date_created BIGINT NOT NULL,
  date_edited BIGINT,
  text VARCHAR NOT NULL,
  CONSTRAINT fk_torrents
    FOREIGN KEY (torrent_id)
    REFERENCES torrents(id)
    ON DELETE CASCADE
);
