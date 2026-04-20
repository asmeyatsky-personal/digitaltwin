CREATE SCHEMA IF NOT EXISTS achievement;

CREATE TABLE IF NOT EXISTS achievement.achievements (
    id              UUID PRIMARY KEY,
    key             TEXT UNIQUE NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT NOT NULL,
    category        TEXT NOT NULL,
    required_count  INTEGER NOT NULL CHECK (required_count > 0)
);

CREATE TABLE IF NOT EXISTS achievement.user_achievements (
    user_id         UUID NOT NULL,
    achievement_id  UUID NOT NULL REFERENCES achievement.achievements(id) ON DELETE CASCADE,
    progress        INTEGER NOT NULL DEFAULT 0,
    unlocked_at     TIMESTAMPTZ,
    PRIMARY KEY (user_id, achievement_id)
);

CREATE INDEX IF NOT EXISTS user_achievements_user_idx ON achievement.user_achievements(user_id);
