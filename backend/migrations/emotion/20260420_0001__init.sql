CREATE SCHEMA IF NOT EXISTS emotion;

CREATE TABLE IF NOT EXISTS emotion.readings (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL,
    modality        TEXT NOT NULL CHECK (modality IN ('face', 'voice', 'text', 'biometric')),
    tone            TEXT NOT NULL,
    confidence      REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
    recorded_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS readings_user_recorded_idx
    ON emotion.readings (user_id, recorded_at DESC);
