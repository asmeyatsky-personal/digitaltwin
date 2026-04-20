CREATE SCHEMA IF NOT EXISTS voice;

CREATE TABLE IF NOT EXISTS voice.profiles (
    user_id          UUID PRIMARY KEY,
    sample_url       TEXT NOT NULL,
    cloned_voice_id  TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS voice.jobs (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL,
    text        TEXT NOT NULL,
    emotion     TEXT NOT NULL,
    status      TEXT NOT NULL CHECK (status IN ('queued','complete','failed')),
    audio_url   TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS jobs_user_idx ON voice.jobs(user_id, created_at DESC);
