-- Phase 34: an open-amount Tikkie link per event, shown before bank
-- transfer until the end of tikkie_valid_until in Amsterdam
-- (requirement.md Decisions 12-14).
ALTER TABLE events
    ADD COLUMN tikkie_url TEXT,
    ADD COLUMN tikkie_valid_until DATE,
    ADD CONSTRAINT events_tikkie_pair
        CHECK ((tikkie_url IS NULL) = (tikkie_valid_until IS NULL));
