DROP INDEX idx_categories_pending;
DROP INDEX idx_menu_items_pending;

ALTER TABLE categories
    DROP COLUMN published_at,
    DROP COLUMN draft_updated_at,
    DROP COLUMN draft;

ALTER TABLE menu_items
    DROP COLUMN published_at,
    DROP COLUMN draft_updated_at,
    DROP COLUMN draft;

DROP TABLE site_content;
