-- Phase 20: the order lifecycle is reworked around bank-transfer payment
-- (requirement.md Decisions 1, 2, 4). Existing orders are mapped onto the
-- new statuses before the new CHECK goes on.
ALTER TABLE orders DROP CONSTRAINT orders_status_check;

UPDATE orders SET status = CASE
    WHEN status = 'new' THEN 'awaiting_payment'
    WHEN status IN ('confirmed', 'preparing') THEN 'processing'
    WHEN status = 'ready' AND delivery_type = 'pickup' THEN 'ready_for_collection'
    WHEN status = 'ready' THEN 'out_for_delivery'
    ELSE status
END;

ALTER TABLE orders
    ALTER COLUMN status SET DEFAULT 'awaiting_payment',
    ADD CONSTRAINT orders_status_check CHECK (status IN (
        'awaiting_payment', 'payment_claimed', 'processing',
        'ready_for_collection', 'out_for_delivery', 'completed', 'cancelled')),
    ADD COLUMN public_token UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    ADD COLUMN payment_claimed_at TIMESTAMPTZ,
    ADD COLUMN payment_confirmed_at TIMESTAMPTZ;
