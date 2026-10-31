-- Phase 22: step timestamps for the tracking page, who completed the
-- order, and the customer's locale for status emails (requirement.md
-- Decisions 4, 5, 8).
ALTER TABLE orders
    ADD COLUMN locale TEXT NOT NULL DEFAULT 'en'
        CONSTRAINT orders_locale_check CHECK (locale IN ('en', 'nl')),
    ADD COLUMN ready_at TIMESTAMPTZ,
    ADD COLUMN completed_at TIMESTAMPTZ,
    ADD COLUMN cancelled_at TIMESTAMPTZ,
    ADD COLUMN completed_by TEXT
        CONSTRAINT orders_completed_by_check CHECK (completed_by IN ('customer', 'admin'));
