CREATE SCHEMA IF NOT EXISTS avatar;

CREATE TABLE IF NOT EXISTS avatar.jobs (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL,
    photo_url       TEXT NOT NULL,
    status          TEXT NOT NULL CHECK (status IN ('queued','processing','complete','failed')),
    result_url      TEXT,
    failure_reason  TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS jobs_user_idx ON avatar.jobs(user_id, created_at DESC);
