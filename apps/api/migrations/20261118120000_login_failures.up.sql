-- Phase 33 Decision 3: per-email admin login lockout. Rows older than a
-- day are deleted by the retention sweeper (retention.rs).
CREATE TABLE login_failures (
    id BIGSERIAL PRIMARY KEY,
    email TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_login_failures_email_time ON login_failures (lower(email), created_at);
