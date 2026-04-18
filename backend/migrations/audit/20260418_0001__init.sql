-- Audit ledger. Separate schema owned by a dedicated role with INSERT-only
-- grants (§4 "separate IAM"). Append-only by construction — no UPDATE/DELETE
-- grants are issued to any role.

CREATE SCHEMA IF NOT EXISTS audit;

CREATE TABLE IF NOT EXISTS audit.events (
    id              BIGSERIAL PRIMARY KEY,
    occurred_at     TIMESTAMPTZ NOT NULL,
    actor_id        UUID        NOT NULL,
    action          TEXT        NOT NULL,
    entity_type     TEXT        NOT NULL,
    entity_id       TEXT        NOT NULL,
    before_hash     TEXT        NOT NULL,
    after_hash      TEXT        NOT NULL
);

CREATE INDEX IF NOT EXISTS events_occurred_at_idx ON audit.events (occurred_at);
CREATE INDEX IF NOT EXISTS events_entity_idx      ON audit.events (entity_type, entity_id);

-- Role + grants. Run as a superuser during provisioning.
-- CREATE ROLE audit_writer LOGIN;
-- GRANT USAGE ON SCHEMA audit TO audit_writer;
-- GRANT INSERT ON audit.events TO audit_writer;
-- REVOKE UPDATE, DELETE ON audit.events FROM PUBLIC;
