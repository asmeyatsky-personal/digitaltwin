CREATE SCHEMA IF NOT EXISTS creative;

CREATE TABLE IF NOT EXISTS creative.works (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL,
    work_type  TEXT NOT NULL CHECK (work_type IN ('story','poem','reflection','gratitude','other')),
    title      TEXT NOT NULL,
    content    TEXT NOT NULL,
    mood       TEXT NOT NULL DEFAULT '',
    is_shared  BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS works_user_idx ON creative.works(user_id, created_at DESC);
