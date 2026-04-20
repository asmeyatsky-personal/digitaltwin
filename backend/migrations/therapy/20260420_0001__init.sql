CREATE SCHEMA IF NOT EXISTS therapy;

CREATE TABLE IF NOT EXISTS therapy.therapists (
    id               UUID PRIMARY KEY,
    name             TEXT NOT NULL,
    credentials      TEXT NOT NULL DEFAULT '',
    specializations  JSONB NOT NULL DEFAULT '[]',
    rate_per_session INTEGER NOT NULL,
    is_verified      BOOLEAN NOT NULL DEFAULT false,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS therapy.screenings (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL,
    screening_type  TEXT NOT NULL CHECK (screening_type IN ('PHQ9','GAD7')),
    responses       JSONB NOT NULL,
    score           INTEGER NOT NULL,
    severity        TEXT NOT NULL CHECK (severity IN ('none','mild','moderate','moderately_severe','severe')),
    completed_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS screenings_user_idx ON therapy.screenings(user_id, completed_at DESC);
