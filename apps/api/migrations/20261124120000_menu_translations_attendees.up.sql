-- Phase 38: per-locale menu text (requirement.md Decision 1), DeepL
-- settings and the keep list (Decisions 6, 20), localized order-line
-- snapshots (Decision 9) and the once-only attendee email (Decision 17).
-- {"nl": {"name", "description", "source": {...English...}, "manual", "attempts"}, "fr": {...}}
ALTER TABLE menu_items
    ADD COLUMN translations JSONB NOT NULL DEFAULT '{}' CHECK (jsonb_typeof(translations) = 'object');
ALTER TABLE categories
    ADD COLUMN translations JSONB NOT NULL DEFAULT '{}' CHECK (jsonb_typeof(translations) = 'object');
ALTER TABLE menu_item_price_options
    ADD COLUMN translations JSONB NOT NULL DEFAULT '{}' CHECK (jsonb_typeof(translations) = 'object');
ALTER TABLE rentals
    ADD COLUMN translations JSONB NOT NULL DEFAULT '{}' CHECK (jsonb_typeof(translations) = 'object');

CREATE TABLE translation_settings (
    id BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    -- Decision 6: whole words DeepL must leave alone.
    keep_words TEXT[] NOT NULL DEFAULT '{}',
    -- Bumped when keep_words changes; a translation made under an older
    -- version is stale (Decision 6), so no row needs rewriting.
    keep_version INT NOT NULL DEFAULT 1,
    -- Decision 20: the owner's switch and the stop line on DeepL's allowance.
    auto_translate BOOLEAN NOT NULL DEFAULT true,
    stop_at_percent INT NOT NULL DEFAULT 80 CHECK (stop_at_percent BETWEEN 10 AND 100),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO translation_settings (keep_words) VALUES (ARRAY[
    'Jollof', 'Egusi', 'Efo Riro', 'Ogbono', 'Ofada', 'Puff Puff',
    'Small Chops', 'Suya', 'Chin Chin', 'Abula', 'Amala', 'Gbegiri',
    'Fufu', 'Moi Moi', 'Dodo', 'Asun', 'Zobo', 'Meat Pie'
]);

ALTER TABLE order_items
    ADD COLUMN item_name_local TEXT,
    ADD COLUMN option_label_local TEXT;

ALTER TABLE events ADD COLUMN attendees_sent_at TIMESTAMPTZ;
