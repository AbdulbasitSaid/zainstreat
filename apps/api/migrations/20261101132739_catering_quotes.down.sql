-- Fails if any on_site / serving_on_site orders exist; a down migration is
-- for dev databases.
ALTER TABLE orders
    DROP CONSTRAINT orders_status_check,
    ADD CONSTRAINT orders_status_check CHECK (status IN (
        'awaiting_payment', 'payment_claimed', 'processing',
        'ready_for_collection', 'out_for_delivery', 'completed', 'cancelled')),
    DROP CONSTRAINT orders_delivery_type_check,
    ADD CONSTRAINT orders_delivery_type_check
        CHECK (delivery_type IN ('pickup', 'delivery'));

DROP TABLE catering_quote_items;
DROP TABLE catering_quotes;

ALTER TABLE catering_enquiries DROP COLUMN locale;
