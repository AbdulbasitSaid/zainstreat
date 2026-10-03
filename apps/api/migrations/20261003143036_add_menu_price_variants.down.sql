DROP TABLE menu_item_price_options;

-- `price` is intentionally left NULLable on the way down: once real rows
-- exist with price options (price IS NULL), restoring NOT NULL here would
-- fail outright. Reverting in an environment with real price-variant data
-- requires a manual backfill first — this migrator is only ever run by
-- hand in local dev, same precedent as the initial migration's own
-- down.sql.
ALTER TABLE menu_items DROP COLUMN display_order;

ALTER TABLE categories DROP COLUMN display_order;
