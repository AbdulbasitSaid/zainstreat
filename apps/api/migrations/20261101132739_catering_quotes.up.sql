-- Phase 23: catering quotations (requirement.md Decisions 1-7, 12, 13).
-- A quote covers the quote stage only; accepting it creates a normal order.
ALTER TABLE catering_enquiries
    ADD COLUMN locale TEXT NOT NULL DEFAULT 'en'
        CONSTRAINT catering_enquiries_locale_check CHECK (locale IN ('en', 'nl'));

CREATE TABLE catering_quotes (
    id BIGSERIAL PRIMARY KEY,
    enquiry_id BIGINT NOT NULL UNIQUE REFERENCES catering_enquiries(id) ON DELETE RESTRICT,
    public_token UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'sent', 'accepted', 'declined')),
    delivery_type TEXT NOT NULL CHECK (delivery_type IN ('pickup', 'delivery', 'on_site')),
    delivery_address TEXT,
    notes TEXT,
    total NUMERIC(10, 2) NOT NULL CHECK (total > 0),
    valid_until DATE NOT NULL,
    -- Bumped on every admin save; accept/decline must echo it (Decision 13).
    version INTEGER NOT NULL DEFAULT 1,
    sent_at TIMESTAMPTZ,
    accepted_at TIMESTAMPTZ,
    declined_at TIMESTAMPTZ,
    decline_reason TEXT,
    order_id BIGINT UNIQUE REFERENCES orders(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT catering_quotes_order_when_accepted
        CHECK ((status = 'accepted') = (order_id IS NOT NULL))
);

CREATE TABLE catering_quote_items (
    id BIGSERIAL PRIMARY KEY,
    quote_id BIGINT NOT NULL REFERENCES catering_quotes(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    description TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price NUMERIC(10, 2) NOT NULL CHECK (unit_price >= 0),
    subtotal NUMERIC(10, 2) NOT NULL,
    UNIQUE (quote_id, position)
);

-- On-site fulfilment for catering orders (Decision 12).
ALTER TABLE orders
    DROP CONSTRAINT orders_delivery_type_check,
    ADD CONSTRAINT orders_delivery_type_check
        CHECK (delivery_type IN ('pickup', 'delivery', 'on_site')),
    DROP CONSTRAINT orders_status_check,
    ADD CONSTRAINT orders_status_check CHECK (status IN (
        'awaiting_payment', 'payment_claimed', 'processing',
        'ready_for_collection', 'out_for_delivery', 'serving_on_site',
        'completed', 'cancelled'));
