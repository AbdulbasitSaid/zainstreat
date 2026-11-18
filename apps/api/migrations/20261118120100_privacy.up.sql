-- Phase 33 Decisions 15-16: when a row's personal data was anonymised
-- (also keeps the retention sweeper idempotent), and the log of the admin's
-- export/erasure requests, keyed by a hash so the log holds no email.
ALTER TABLE orders ADD COLUMN anonymised_at TIMESTAMPTZ;
ALTER TABLE event_bookings ADD COLUMN anonymised_at TIMESTAMPTZ;
ALTER TABLE catering_enquiries ADD COLUMN anonymised_at TIMESTAMPTZ;

CREATE TABLE privacy_requests (
    id BIGSERIAL PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('export', 'anonymise')),
    email_sha256 TEXT NOT NULL,
    row_counts JSONB NOT NULL,
    admin_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
