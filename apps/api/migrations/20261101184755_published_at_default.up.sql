-- Phase 25: rows inserted by hand (seed.sql, the production menu seed,
-- tests) are live by default; the admin create endpoints insert NULL
-- explicitly, so rows created in admin still start unpublished.
ALTER TABLE menu_items ALTER COLUMN published_at SET DEFAULT now();
ALTER TABLE categories ALTER COLUMN published_at SET DEFAULT now();
