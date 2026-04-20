CREATE SCHEMA IF NOT EXISTS family;

CREATE TABLE IF NOT EXISTS family.families (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    created_by  UUID NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS family.members (
    family_id   UUID NOT NULL REFERENCES family.families(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL,
    role        TEXT NOT NULL CHECK (role IN ('owner','adult','child')),
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (family_id, user_id)
);

CREATE INDEX IF NOT EXISTS members_user_idx ON family.members(user_id);
