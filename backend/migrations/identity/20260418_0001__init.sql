-- Identity bounded context. Migration runs against the `identity` schema.
-- Postgres primary per ADR-0003; Firestore is for document-shaped data only.

CREATE SCHEMA IF NOT EXISTS identity;

CREATE TABLE IF NOT EXISTS identity.users (
    id              UUID PRIMARY KEY,
    email           CITEXT NOT NULL,
    password_hash   TEXT   NOT NULL,
    status          TEXT   NOT NULL CHECK (status IN ('active', 'suspended', 'deleted')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS users_email_uk ON identity.users (email);

-- Revoked refresh tokens (§4: every external call has circuit breaker; this
-- is the persistent blacklist the prior AUDIT flagged as in-memory only).
CREATE TABLE IF NOT EXISTS identity.revoked_tokens (
    jti             TEXT PRIMARY KEY,
    revoked_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS revoked_tokens_expiry_idx ON identity.revoked_tokens (expires_at);
