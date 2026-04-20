CREATE SCHEMA IF NOT EXISTS moderation;

CREATE TABLE IF NOT EXISTS moderation.reports (
    id            UUID PRIMARY KEY,
    reporter      UUID NOT NULL,
    content_type  TEXT NOT NULL,
    content_id    TEXT NOT NULL,
    reason        TEXT NOT NULL CHECK (reason IN ('harassment','spam','self_harm','inappropriate','other')),
    status        TEXT NOT NULL CHECK (status IN ('pending','reviewed','actioned','dismissed')),
    reviewed_by   UUID,
    notes         TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS reports_status_idx ON moderation.reports(status, created_at);
