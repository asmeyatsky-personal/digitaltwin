CREATE SCHEMA IF NOT EXISTS learning;

CREATE TABLE IF NOT EXISTS learning.paths (
    id                 UUID PRIMARY KEY,
    title              TEXT NOT NULL,
    description        TEXT NOT NULL DEFAULT '',
    category           TEXT NOT NULL,
    modules            JSONB NOT NULL,
    estimated_minutes  INTEGER NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS learning.progress (
    user_id           UUID NOT NULL,
    path_id           UUID NOT NULL REFERENCES learning.paths(id) ON DELETE CASCADE,
    current_module    INTEGER NOT NULL DEFAULT 0,
    reflection_notes  TEXT NOT NULL DEFAULT '',
    started_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at      TIMESTAMPTZ,
    PRIMARY KEY (user_id, path_id)
);
