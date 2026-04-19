-- Conversation bounded context. Postgres holds conversation metadata;
-- message transcripts land here in JSONB today and migrate to Firestore
-- (ADR-0003) as part of the Firestore client wiring.

CREATE SCHEMA IF NOT EXISTS conversation;

CREATE TABLE IF NOT EXISTS conversation.conversations (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL,
    title           TEXT,
    status          TEXT NOT NULL CHECK (status IN ('active', 'ended')),
    message_count   INTEGER NOT NULL DEFAULT 0,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at        TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS conversations_user_id_idx
    ON conversation.conversations (user_id, started_at DESC);

CREATE TABLE IF NOT EXISTS conversation.messages (
    id                  UUID PRIMARY KEY,
    conversation_id     UUID NOT NULL REFERENCES conversation.conversations(id) ON DELETE CASCADE,
    role                TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    body                TEXT NOT NULL,
    tone                TEXT NOT NULL,
    sent_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS messages_conversation_idx
    ON conversation.messages (conversation_id, sent_at ASC);
