CREATE SCHEMA IF NOT EXISTS community;

CREATE TABLE IF NOT EXISTS community.groups (
    id           UUID PRIMARY KEY,
    name         TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    category     TEXT NOT NULL,
    is_moderated BOOLEAN NOT NULL DEFAULT true,
    created_by   UUID NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS community.memberships (
    group_id   UUID NOT NULL REFERENCES community.groups(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL,
    joined_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (group_id, user_id)
);

CREATE TABLE IF NOT EXISTS community.posts (
    id             UUID PRIMARY KEY,
    group_id       UUID NOT NULL REFERENCES community.groups(id) ON DELETE CASCADE,
    author         UUID NOT NULL,
    content        TEXT NOT NULL,
    is_anonymous   BOOLEAN NOT NULL DEFAULT false,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS posts_group_idx ON community.posts(group_id, created_at DESC);
CREATE INDEX IF NOT EXISTS memberships_user_idx ON community.memberships(user_id);
