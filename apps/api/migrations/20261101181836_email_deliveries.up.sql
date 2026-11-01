-- Phase 25: record customer email outcomes so admin can flag failures
-- (requirement.md Decision 6a).
CREATE TABLE email_deliveries (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT REFERENCES orders(id) ON DELETE CASCADE,
    quote_id BIGINT REFERENCES catering_quotes(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('invoice', 'status', 'quote')),
    recipient TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'sent', 'failed')),
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((order_id IS NULL) <> (quote_id IS NULL))
);
CREATE INDEX idx_email_deliveries_order ON email_deliveries (order_id) WHERE order_id IS NOT NULL;
CREATE INDEX idx_email_deliveries_quote ON email_deliveries (quote_id) WHERE quote_id IS NOT NULL;
