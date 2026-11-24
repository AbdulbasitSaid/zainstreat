ALTER TABLE events DROP COLUMN attendees_sent_at;
ALTER TABLE order_items
    DROP COLUMN option_label_local,
    DROP COLUMN item_name_local;
DROP TABLE translation_settings;
ALTER TABLE rentals DROP COLUMN translations;
ALTER TABLE menu_item_price_options DROP COLUMN translations;
ALTER TABLE categories DROP COLUMN translations;
ALTER TABLE menu_items DROP COLUMN translations;
