-- Admin web push subscriptions (Phase 27 Decision 10): one row per device
-- (browser push endpoint), owned by the admin who turned notifications on.
CREATE TABLE push_subscriptions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL UNIQUE,
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_success_at TIMESTAMPTZ,
    last_failure_at TIMESTAMPTZ,
    failure_count INT NOT NULL DEFAULT 0
);

CREATE INDEX push_subscriptions_user_id_idx ON push_subscriptions (user_id);
