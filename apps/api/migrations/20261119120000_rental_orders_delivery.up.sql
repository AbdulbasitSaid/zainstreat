-- Phase 37: rentals in orders (requirement.md Decisions 3, 4) and the
-- delivery fee (Decisions 5, 6, 12, 17).
ALTER TABLE order_items
    ADD COLUMN rental_id BIGINT REFERENCES rentals(id) ON DELETE SET NULL,
    -- Kept apart from rental_id so a line stays a rental after its rental is deleted.
    ADD COLUMN kind TEXT NOT NULL DEFAULT 'menu' CHECK (kind IN ('menu', 'rental')),
    ADD CONSTRAINT order_items_one_source
        CHECK (menu_item_id IS NULL OR rental_id IS NULL);

ALTER TABLE orders
    ADD COLUMN rental_date DATE,
    ADD COLUMN delivery_distance_km NUMERIC(6, 1),
    ADD COLUMN delivery_rate_per_km NUMERIC(10, 2),
    ADD COLUMN delivery_estimated_by TEXT
        CHECK (delivery_estimated_by IN ('route', 'straight_line', 'manual')),
    -- Decision 17: placed while address lookup was down; payment is held
    -- until the owner sets the fee.
    ADD COLUMN delivery_fee_pending BOOLEAN NOT NULL DEFAULT false;

CREATE TABLE delivery_settings (
    id BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    -- Private: only ever returned by the admin API (Decision 7).
    origin_address TEXT,
    origin_lat DOUBLE PRECISION,
    origin_lon DOUBLE PRECISION,
    rate_per_km NUMERIC(10, 2) NOT NULL DEFAULT 1.00 CHECK (rate_per_km >= 0),
    minimum_fee NUMERIC(10, 2) NOT NULL DEFAULT 0.00 CHECK (minimum_fee >= 0),
    max_distance_km NUMERIC(6, 1) CHECK (max_distance_km > 0),
    -- Decision 6: off until the owner has checked the seeded address.
    delivery_enabled BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (NOT delivery_enabled OR (origin_lat IS NOT NULL AND origin_lon IS NOT NULL))
);

-- Decision 6: the business address starts as the current event's address
-- (the next upcoming published event, else the newest with an address).
INSERT INTO delivery_settings (origin_address)
SELECT (
    SELECT address FROM events
    WHERE archived_at IS NULL AND btrim(address) <> ''
    ORDER BY (published_at IS NOT NULL AND starts_at >= now()) DESC,
             CASE WHEN published_at IS NOT NULL AND starts_at >= now() THEN starts_at END ASC,
             created_at DESC
    LIMIT 1
);
