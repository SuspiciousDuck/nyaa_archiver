ALTER TABLE torrents ADD update_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE torrents ADD update_frequency INTEGER;
CREATE INDEX idx_torrents_update_count ON torrents(update_count);
CREATE INDEX idx_torrents_next_update ON torrents(next_update);
