-- Add explicit ordering to categories and menu items (replaces the
-- implicit "ORDER BY id" convention from the initial schema), and
-- introduce per-item price variants (sizes/options) as a child table.
--
-- Invariant (application-level only, NOT enforced by a DB constraint —
-- Postgres CHECK constraints can't reference another table): a
-- menu_items row has EITHER a non-NULL `price` and zero rows in
-- menu_item_price_options, OR a NULL `price` and one-or-more rows in
-- menu_item_price_options — never both, never neither. Enforced by
-- seed.sql's construction today; the admin menu-editing phase should
-- validate this invariant client- and server-side when it's built.

ALTER TABLE categories
    ADD COLUMN display_order INT NOT NULL DEFAULT 0;

ALTER TABLE menu_items
    ADD COLUMN display_order INT NOT NULL DEFAULT 0,
    ALTER COLUMN price DROP NOT NULL;

CREATE TABLE menu_item_price_options (
    id BIGSERIAL PRIMARY KEY,
    menu_item_id BIGINT NOT NULL REFERENCES menu_items(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    price NUMERIC(10, 2) NOT NULL,
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);
CREATE INDEX idx_menu_item_price_options_menu_item_id
    ON menu_item_price_options(menu_item_id);
