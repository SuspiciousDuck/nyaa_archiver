ALTER TABLE torrents DROP next_update;
ALTER TABLE torrents ADD last_updated BIGINT
