-- Phase 25: draft → preview → publish for site content and the menu
-- (requirement.md Decisions 11, 15).
CREATE TABLE site_content (
    id BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    published JSONB NOT NULL DEFAULT '{}'::jsonb,
    draft JSONB,
    draft_updated_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO site_content DEFAULT VALUES;

ALTER TABLE menu_items
    ADD COLUMN draft JSONB,
    ADD COLUMN draft_updated_at TIMESTAMPTZ,
    ADD COLUMN published_at TIMESTAMPTZ;
UPDATE menu_items SET published_at = created_at;

ALTER TABLE categories
    ADD COLUMN draft JSONB,
    ADD COLUMN draft_updated_at TIMESTAMPTZ,
    ADD COLUMN published_at TIMESTAMPTZ;
UPDATE categories SET published_at = created_at;

CREATE INDEX idx_menu_items_pending
    ON menu_items (id) WHERE deleted_at IS NULL AND (draft IS NOT NULL OR published_at IS NULL);
CREATE INDEX idx_categories_pending
    ON categories (id) WHERE deleted_at IS NULL AND (draft IS NOT NULL OR published_at IS NULL);
