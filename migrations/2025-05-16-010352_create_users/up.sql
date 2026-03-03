CREATE TABLE users (
  username VARCHAR NOT NULL PRIMARY KEY COLLATE NOCASE UNIQUE,
  password VARCHAR,
  salt VARCHAR,
  email VARCHAR,
  nyaa BOOLEAN NOT NULL DEFAULT FALSE
  CHECK (
    nyaa = TRUE OR password IS NOT NULL
  )
  CHECK (
    nyaa = TRUE or email IS NOT NULL
  )
  CHECK (
    password = NULL OR salt IS NOT NULL
  )
)
