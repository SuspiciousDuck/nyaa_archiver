ALTER TABLE torrents ADD next_update BIGINT;
ALTER TABLE torrents DROP last_updated;

