-- Phase 32: per-plate option groups (protein, side) and each guest's
-- choices (requirement.md Decisions 1-4). Choices don't change the price.
-- [{"key", "label": {"en", "nl", "fr"}, "choices": [{"key", "label": {...}}]}]
ALTER TABLE event_plates
    ADD COLUMN options JSONB NOT NULL DEFAULT '[]'
        CHECK (jsonb_typeof(options) = 'array');

-- [{"group", "choice", "label"}], in group order, label in English.
ALTER TABLE event_booking_guests
    ADD COLUMN choices JSONB NOT NULL DEFAULT '[]'
        CHECK (jsonb_typeof(choices) = 'array');
