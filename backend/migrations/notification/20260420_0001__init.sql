CREATE SCHEMA IF NOT EXISTS notification;

CREATE TABLE IF NOT EXISTS notification.tokens (
    token       TEXT PRIMARY KEY,
    user_id     UUID NOT NULL,
    platform    TEXT NOT NULL CHECK (platform IN ('ios','android','web')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS tokens_user_idx ON notification.tokens(user_id);
