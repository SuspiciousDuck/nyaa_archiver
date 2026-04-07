ALTER TABLE torrents DROP update_count;
ALTER TABLE torrents DROP update_frequency;
DROP INDEX idx_torrents_update_count;
DROP INDEX idx_torrents_next_update;
