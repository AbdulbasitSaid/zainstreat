ALTER TABLE orders
    DROP CONSTRAINT orders_status_check,
    DROP COLUMN public_token,
    DROP COLUMN payment_claimed_at,
    DROP COLUMN payment_confirmed_at;

UPDATE orders SET status = CASE
    WHEN status IN ('awaiting_payment', 'payment_claimed') THEN 'new'
    WHEN status = 'processing' THEN 'preparing'
    WHEN status IN ('ready_for_collection', 'out_for_delivery') THEN 'ready'
    ELSE status
END;

ALTER TABLE orders
    ALTER COLUMN status SET DEFAULT 'new',
    ADD CONSTRAINT orders_status_check
        CHECK (status IN ('new', 'confirmed', 'preparing', 'ready', 'completed', 'cancelled'));
