CREATE TABLE torrents (
  id INTEGER NOT NULL PRIMARY KEY UNIQUE,
  info_hash VARCHAR,
  seeders INTEGER NOT NULL DEFAULT 0,
  leechers INTEGER NOT NULL DEFAULT 0,
  completed INTEGER NOT NULL DEFAULT 0,
  title VARCHAR NOT NULL,
  category INTEGER NOT NULL,
  submitter VARCHAR,
  information VARCHAR DEFAULT '',
  size BIGINT NOT NULL,
  date INTEGER NOT NULL,
  description VARCHAR DEFAULT '',
  comments INTEGER NOT NULL DEFAULT 0,
  remake BOOLEAN NOT NULL DEFAULT FALSE,
  trusted BOOLEAN NOT NULL DEFAULT FALSE,
  partial BOOLEAN NOT NULL DEFAULT TRUE
  CHECK (
    partial = TRUE or info_hash IS NOT NULL
  )
  CHECK (
    partial = TRUE or information IS NOT NULL
  )
  CHECK (
    partial = TRUE or description IS NOT NULL
  )
);
