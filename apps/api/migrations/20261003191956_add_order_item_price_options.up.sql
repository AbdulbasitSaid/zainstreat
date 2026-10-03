ALTER TABLE order_items
    ADD COLUMN price_option_id BIGINT REFERENCES menu_item_price_options(id) ON DELETE SET NULL,
    ADD COLUMN option_label TEXT;
