-- Phase 36: per-item minimum order (requirement.md Decision 2) and
-- admin-managed rentals (Decision 8). Rentals are never ordered through
-- the cart, so they get their own table instead of being menu items.
ALTER TABLE menu_items
    ADD COLUMN min_quantity INT NOT NULL DEFAULT 1
        CHECK (min_quantity BETWEEN 1 AND 999);

CREATE TABLE rentals (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    price NUMERIC(10, 2) NOT NULL CHECK (price >= 0),
    image_url TEXT,
    is_available BOOLEAN NOT NULL DEFAULT true,
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);
