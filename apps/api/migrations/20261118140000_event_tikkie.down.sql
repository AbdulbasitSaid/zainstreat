ALTER TABLE events
    DROP CONSTRAINT events_tikkie_pair,
    DROP COLUMN tikkie_valid_until,
    DROP COLUMN tikkie_url;
