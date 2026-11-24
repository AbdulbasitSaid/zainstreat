-- Phase 38 Decision 22: the admin closes an event's bookings by hand. NULL
-- is "not closed by hand"; LEAST() ignores it, so close checks are unchanged.
ALTER TABLE events ADD COLUMN closed_at TIMESTAMPTZ;
