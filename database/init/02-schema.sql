-- ============================================================
-- Digital Twin - Database Schema
-- All tables derived from DigitalTwinDbContext OnModelCreating
-- Executed on first database initialization
-- ============================================================

-- ============================================================
-- AI Twin Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS ai_twin_profiles (
    id                   UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name                 VARCHAR(200)  NOT NULL,
    description          VARCHAR(1000),
    user_id              VARCHAR(50)   NOT NULL,
    building_id          UUID          NOT NULL,
    creation_date        TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_interaction     TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    training_last_run    TIMESTAMP,
    learning_mode        INTEGER       NOT NULL DEFAULT 0,
    personality_traits   TEXT,          -- JSON: AITwinPersonalityTraits
    behavioral_patterns  TEXT,          -- JSON: Dictionary<string, double>
    preferences          TEXT,          -- JSON: Dictionary<string, object>
    activation_level     DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    emotional_state      INTEGER       NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS ix_ai_twin_profiles_user_id
    ON ai_twin_profiles (user_id);

-- ---

CREATE TABLE IF NOT EXISTS ai_twin_knowledge (
    id                   UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    type                 INTEGER       NOT NULL DEFAULT 0,
    content              TEXT          NOT NULL,
    importance           DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    confidence           DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    creation_date        TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_updated         TIMESTAMP,
    source               VARCHAR(500),
    tags                 TEXT,          -- JSON: List<string>
    ai_twin_profile_id   UUID,
    CONSTRAINT fk_ai_twin_knowledge_profile
        FOREIGN KEY (ai_twin_profile_id)
        REFERENCES ai_twin_profiles (id)
        ON DELETE CASCADE
);

-- ---

CREATE TABLE IF NOT EXISTS ai_twin_interactions (
    id                   UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    twin_id              UUID          NOT NULL,
    message_type         VARCHAR(100)  NOT NULL,
    content              TEXT          NOT NULL,
    timestamp            TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    context              TEXT,          -- JSON: Dictionary<string, object>
    emotional_tone       INTEGER       NOT NULL DEFAULT 0,
    -- Owned entity: AITwinInteractionResponse
    response_content     TEXT          NOT NULL DEFAULT '',
    response_emotional_tone INTEGER    NOT NULL DEFAULT 0,
    response_confidence  DOUBLE PRECISION NOT NULL DEFAULT 0,
    response_timestamp   TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ai_twin_profile_id   UUID,
    CONSTRAINT fk_ai_twin_interactions_profile
        FOREIGN KEY (ai_twin_profile_id)
        REFERENCES ai_twin_profiles (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS ix_ai_twin_interactions_twin_id
    ON ai_twin_interactions (twin_id);

CREATE INDEX IF NOT EXISTS ix_ai_twin_interactions_timestamp
    ON ai_twin_interactions (timestamp);

-- ---

CREATE TABLE IF NOT EXISTS ai_twin_memories (
    id                        UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    type                      INTEGER       NOT NULL DEFAULT 0,
    content                   TEXT          NOT NULL,
    importance                DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    creation_date             TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    associated_interactions   TEXT,          -- JSON: List<Guid>
    emotional_valence         DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    tags                      TEXT,          -- JSON: List<string>
    ai_twin_profile_id        UUID,
    CONSTRAINT fk_ai_twin_memories_profile
        FOREIGN KEY (ai_twin_profile_id)
        REFERENCES ai_twin_profiles (id)
        ON DELETE CASCADE
);


-- ============================================================
-- Conversation Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS conversation_sessions (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  VARCHAR(50)  NOT NULL,
    start_time               TIMESTAMP    NOT NULL,
    started_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    end_time                 TIMESTAMP,
    ended_at                 TIMESTAMP,
    last_message_at          TIMESTAMP,
    dominant_emotion         INTEGER      NOT NULL DEFAULT 0,
    current_emotional_state  INTEGER      NOT NULL DEFAULT 0,
    message_count            INTEGER      NOT NULL DEFAULT 0,
    session_context          TEXT,         -- JSON: Dictionary<string, object>
    conversation_context     TEXT,         -- JSON: Dictionary<string, object>
    is_active                BOOLEAN      NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS ix_conversation_sessions_user_id
    ON conversation_sessions (user_id);

CREATE INDEX IF NOT EXISTS ix_conversation_sessions_is_active
    ON conversation_sessions (is_active);

-- ---

CREATE TABLE IF NOT EXISTS conversation_messages (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    role                     VARCHAR(20)  NOT NULL,
    content                  TEXT,
    timestamp                TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    detected_emotion         INTEGER      NOT NULL DEFAULT 0,
    encrypted_content        BYTEA,
    iv                       BYTEA,
    auth_tag                 BYTEA,
    is_encrypted             BOOLEAN      NOT NULL DEFAULT FALSE,
    conversation_session_id  UUID,
    CONSTRAINT fk_conversation_messages_session
        FOREIGN KEY (conversation_session_id)
        REFERENCES conversation_sessions (id)
        ON DELETE CASCADE
);

-- ---

CREATE TABLE IF NOT EXISTS conversation_memories (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  VARCHAR(50)  NOT NULL,
    key                      VARCHAR(200) NOT NULL,
    value                    TEXT         NOT NULL,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_accessed            TIMESTAMP,
    importance               DOUBLE PRECISION NOT NULL DEFAULT 0.5
);

CREATE INDEX IF NOT EXISTS ix_conversation_memories_user_id
    ON conversation_memories (user_id);

CREATE UNIQUE INDEX IF NOT EXISTS ix_conversation_memories_user_id_key
    ON conversation_memories (user_id, key);


-- ============================================================
-- Emotional Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS emotional_memories (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  TEXT         NOT NULL,
    emotion_type             INTEGER      NOT NULL DEFAULT 0,
    primary_emotion          INTEGER      NOT NULL DEFAULT 0,
    intensity                DOUBLE PRECISION NOT NULL DEFAULT 0,
    context                  TEXT,
    trigger                  TEXT,
    description              TEXT,
    timestamp                TIMESTAMP    NOT NULL,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    importance_score         INTEGER      NOT NULL DEFAULT 0,
    associated_emotions      TEXT,         -- JSON: List<Emotion>
    emotion_tags             TEXT,         -- JSON: List<string>
    metadata                 TEXT,         -- JSON: Dictionary<string, object>
    embedding                vector(1536)
);

CREATE INDEX IF NOT EXISTS ix_emotional_memories_embedding
    ON emotional_memories
    USING ivfflat (embedding vector_cosine_ops);


-- ============================================================
-- Subscription Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS subscriptions (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  VARCHAR(50)  NOT NULL,
    stripe_customer_id       VARCHAR(100),
    stripe_subscription_id   VARCHAR(100),
    stripe_price_id          VARCHAR(100),
    tier                     VARCHAR(20)  NOT NULL DEFAULT 'free',
    status                   VARCHAR(20)  NOT NULL DEFAULT 'active',
    current_period_end       TIMESTAMP,
    cancel_at_period_end     BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS ix_subscriptions_user_id
    ON subscriptions (user_id);

CREATE INDEX IF NOT EXISTS ix_subscriptions_stripe_customer_id
    ON subscriptions (stripe_customer_id);

CREATE INDEX IF NOT EXISTS ix_subscriptions_stripe_subscription_id
    ON subscriptions (stripe_subscription_id);


-- ============================================================
-- Biometric Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS biometric_readings (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  VARCHAR(50)  NOT NULL,
    type                     VARCHAR(30)  NOT NULL,
    value                    DOUBLE PRECISION NOT NULL DEFAULT 0,
    unit                     VARCHAR(20),
    source                   VARCHAR(30),
    timestamp                TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_biometric_readings_user_type_ts
    ON biometric_readings (user_id, type, timestamp);


-- ============================================================
-- Coaching Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS goals (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  VARCHAR(50)  NOT NULL,
    title                    VARCHAR(200) NOT NULL,
    description              VARCHAR(2000),
    category                 VARCHAR(50),
    target_date              TIMESTAMP,
    progress                 DOUBLE PRECISION NOT NULL DEFAULT 0,
    status                   VARCHAR(20)  NOT NULL DEFAULT 'active',
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_goals_user_status
    ON goals (user_id, status);

-- ---

CREATE TABLE IF NOT EXISTS journal_entries (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  VARCHAR(50)  NOT NULL,
    content                  TEXT         NOT NULL,
    mood                     VARCHAR(30),
    tags                     TEXT,         -- JSON: List<string>
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_journal_entries_user_created
    ON journal_entries (user_id, created_at);

-- ---

CREATE TABLE IF NOT EXISTS habit_records (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  VARCHAR(50)  NOT NULL,
    habit_name               VARCHAR(100) NOT NULL,
    completed                BOOLEAN      NOT NULL DEFAULT FALSE,
    date                     DATE         NOT NULL DEFAULT CURRENT_DATE
);

CREATE INDEX IF NOT EXISTS ix_habit_records_user_habit_date
    ON habit_records (user_id, habit_name, date);


-- ============================================================
-- Shared Experience Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS shared_rooms (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name                     VARCHAR(200) NOT NULL,
    creator_user_id          VARCHAR(50)  NOT NULL,
    participants             TEXT,         -- JSON: List<string>
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_active                BOOLEAN      NOT NULL DEFAULT TRUE
);

CREATE INDEX IF NOT EXISTS ix_shared_rooms_is_active
    ON shared_rooms (is_active);


-- ============================================================
-- Personal History Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS life_events (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  UUID         NOT NULL,
    title                    VARCHAR(200) NOT NULL,
    description              VARCHAR(4000),
    event_date               TIMESTAMP    NOT NULL,
    category                 VARCHAR(30)  NOT NULL,
    emotional_impact         VARCHAR(30),
    is_recurring             BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_life_events_user_id
    ON life_events (user_id);

CREATE INDEX IF NOT EXISTS ix_life_events_event_date
    ON life_events (event_date);

CREATE INDEX IF NOT EXISTS ix_life_events_user_event_date
    ON life_events (user_id, event_date);

-- ---

CREATE TABLE IF NOT EXISTS personal_contexts (
    id                           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                      UUID         NOT NULL,
    cultural_background          VARCHAR(500),
    communication_preferences    VARCHAR(2000),
    important_people             VARCHAR(4000),
    values                       VARCHAR(4000),
    updated_at                   TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS ix_personal_contexts_user_id
    ON personal_contexts (user_id);


-- ============================================================
-- Achievement Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS achievement_definitions (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key                      VARCHAR(100) NOT NULL,
    title                    VARCHAR(200) NOT NULL,
    description              VARCHAR(1000),
    icon_name                VARCHAR(50),
    category                 VARCHAR(30)  NOT NULL,
    required_count           INTEGER      NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS ix_achievement_definitions_key
    ON achievement_definitions (key);

-- ---

CREATE TABLE IF NOT EXISTS user_achievements (
    id                           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                      UUID         NOT NULL,
    achievement_definition_id    UUID         NOT NULL,
    progress                     INTEGER      NOT NULL DEFAULT 0,
    is_unlocked                  BOOLEAN      NOT NULL DEFAULT FALSE,
    unlocked_at                  TIMESTAMP,
    created_at                   TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_user_achievements_definition
        FOREIGN KEY (achievement_definition_id)
        REFERENCES achievement_definitions (id)
        ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS ix_user_achievements_user_def
    ON user_achievements (user_id, achievement_definition_id);


-- ============================================================
-- Check-In Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS check_in_records (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  VARCHAR(50)  NOT NULL,
    scheduled_at             TIMESTAMP    NOT NULL,
    sent_at                  TIMESTAMP,
    type                     VARCHAR(30)  NOT NULL DEFAULT 'daily',
    emotion_context          VARCHAR(500),
    response                 VARCHAR(2000),
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_check_in_records_user_scheduled
    ON check_in_records (user_id, scheduled_at);


-- ============================================================
-- Family Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS families (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name                     VARCHAR(200) NOT NULL,
    created_by_user_id       UUID         NOT NULL,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_families_created_by
    ON families (created_by_user_id);

-- ---

CREATE TABLE IF NOT EXISTS family_members (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    family_id                UUID         NOT NULL,
    user_id                  UUID         NOT NULL,
    role                     VARCHAR(20)  NOT NULL,
    joined_at                TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_family_members_family
        FOREIGN KEY (family_id)
        REFERENCES families (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS ix_family_members_user_id
    ON family_members (user_id);

CREATE INDEX IF NOT EXISTS ix_family_members_family_id
    ON family_members (family_id);

CREATE UNIQUE INDEX IF NOT EXISTS ix_family_members_family_user
    ON family_members (family_id, user_id);

-- ---

CREATE TABLE IF NOT EXISTS family_invites (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    family_id                UUID         NOT NULL,
    email                    VARCHAR(256) NOT NULL,
    role                     VARCHAR(20)  NOT NULL,
    invite_code              VARCHAR(8)   NOT NULL,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at               TIMESTAMP    NOT NULL,
    is_accepted              BOOLEAN      NOT NULL DEFAULT FALSE,
    CONSTRAINT fk_family_invites_family
        FOREIGN KEY (family_id)
        REFERENCES families (id)
        ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS ix_family_invites_invite_code
    ON family_invites (invite_code);

CREATE INDEX IF NOT EXISTS ix_family_invites_family_id
    ON family_invites (family_id);


-- ============================================================
-- Community Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS community_groups (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name                     VARCHAR(200) NOT NULL,
    description              VARCHAR(4000),
    category                 VARCHAR(30)  NOT NULL,
    is_moderated             BOOLEAN      NOT NULL DEFAULT FALSE,
    created_by_user_id       UUID         NOT NULL,
    member_count             INTEGER      NOT NULL DEFAULT 0,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_community_groups_category
    ON community_groups (category);

CREATE INDEX IF NOT EXISTS ix_community_groups_created_by
    ON community_groups (created_by_user_id);

-- ---

CREATE TABLE IF NOT EXISTS community_posts (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    group_id                 UUID         NOT NULL,
    author_user_id           UUID         NOT NULL,
    title                    VARCHAR(300) NOT NULL,
    content                  TEXT         NOT NULL,
    is_anonymous             BOOLEAN      NOT NULL DEFAULT FALSE,
    like_count               INTEGER      NOT NULL DEFAULT 0,
    reply_count              INTEGER      NOT NULL DEFAULT 0,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_community_posts_group
        FOREIGN KEY (group_id)
        REFERENCES community_groups (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS ix_community_posts_group_id
    ON community_posts (group_id);

CREATE INDEX IF NOT EXISTS ix_community_posts_author
    ON community_posts (author_user_id);

-- ---

CREATE TABLE IF NOT EXISTS community_replies (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    post_id                  UUID         NOT NULL,
    author_user_id           UUID         NOT NULL,
    content                  TEXT         NOT NULL,
    is_anonymous             BOOLEAN      NOT NULL DEFAULT FALSE,
    like_count               INTEGER      NOT NULL DEFAULT 0,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_community_replies_post
        FOREIGN KEY (post_id)
        REFERENCES community_posts (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS ix_community_replies_post_id
    ON community_replies (post_id);

CREATE INDEX IF NOT EXISTS ix_community_replies_author
    ON community_replies (author_user_id);

-- ---

CREATE TABLE IF NOT EXISTS community_memberships (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    group_id                 UUID         NOT NULL,
    user_id                  UUID         NOT NULL,
    role                     VARCHAR(20)  NOT NULL,
    joined_at                TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_community_memberships_group
        FOREIGN KEY (group_id)
        REFERENCES community_groups (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS ix_community_memberships_group_id
    ON community_memberships (group_id);

CREATE INDEX IF NOT EXISTS ix_community_memberships_user_id
    ON community_memberships (user_id);

CREATE UNIQUE INDEX IF NOT EXISTS ix_community_memberships_group_user
    ON community_memberships (group_id, user_id);


-- ============================================================
-- Creative Expression Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS creative_works (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  UUID         NOT NULL,
    type                     VARCHAR(20)  NOT NULL,
    title                    VARCHAR(300) NOT NULL,
    content                  TEXT         NOT NULL,
    mood                     VARCHAR(20)  NOT NULL,
    is_shared                BOOLEAN      NOT NULL DEFAULT FALSE,
    shared_to_group_id       UUID,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_creative_works_user_id
    ON creative_works (user_id);

CREATE INDEX IF NOT EXISTS ix_creative_works_user_type
    ON creative_works (user_id, type);

CREATE INDEX IF NOT EXISTS ix_creative_works_is_shared
    ON creative_works (is_shared);

CREATE INDEX IF NOT EXISTS ix_creative_works_shared_group
    ON creative_works (shared_to_group_id);

-- ---

CREATE TABLE IF NOT EXISTS collaborative_stories (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    room_id                  UUID         NOT NULL,
    title                    VARCHAR(300) NOT NULL,
    created_by_user_id       UUID         NOT NULL,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_collaborative_stories_room
        FOREIGN KEY (room_id)
        REFERENCES shared_rooms (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS ix_collaborative_stories_room_id
    ON collaborative_stories (room_id);

CREATE INDEX IF NOT EXISTS ix_collaborative_stories_created_by
    ON collaborative_stories (created_by_user_id);

-- ---

CREATE TABLE IF NOT EXISTS story_chapters (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    story_id                 UUID         NOT NULL,
    author_user_id           UUID         NOT NULL,
    content                  TEXT         NOT NULL,
    chapter_order            INTEGER      NOT NULL DEFAULT 0,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_story_chapters_story
        FOREIGN KEY (story_id)
        REFERENCES collaborative_stories (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS ix_story_chapters_story_id
    ON story_chapters (story_id);

CREATE INDEX IF NOT EXISTS ix_story_chapters_story_order
    ON story_chapters (story_id, chapter_order);


-- ============================================================
-- Therapy / Clinical Screening Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS therapist_profiles (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  UUID         NOT NULL,
    name                     VARCHAR(200) NOT NULL,
    credentials              VARCHAR(500),
    bio                      VARCHAR(4000),
    specializations          VARCHAR(2000),
    availability             VARCHAR(4000),
    rate_per_session         NUMERIC(10,2) NOT NULL DEFAULT 0,
    is_verified              BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS ix_therapist_profiles_user_id
    ON therapist_profiles (user_id);

CREATE INDEX IF NOT EXISTS ix_therapist_profiles_is_verified
    ON therapist_profiles (is_verified);

-- ---

CREATE TABLE IF NOT EXISTS therapy_sessions (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    therapist_id             UUID         NOT NULL,
    client_user_id           UUID         NOT NULL,
    scheduled_at             TIMESTAMP    NOT NULL,
    duration_minutes         INTEGER      NOT NULL DEFAULT 50,
    status                   VARCHAR(20)  NOT NULL DEFAULT 'Scheduled',
    notes                    VARCHAR(4000),
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_therapy_sessions_therapist
        FOREIGN KEY (therapist_id)
        REFERENCES therapist_profiles (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS ix_therapy_sessions_therapist_id
    ON therapy_sessions (therapist_id);

CREATE INDEX IF NOT EXISTS ix_therapy_sessions_client
    ON therapy_sessions (client_user_id);

CREATE INDEX IF NOT EXISTS ix_therapy_sessions_client_scheduled
    ON therapy_sessions (client_user_id, scheduled_at);

-- ---

CREATE TABLE IF NOT EXISTS clinical_screenings (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  UUID         NOT NULL,
    type                     VARCHAR(10)  NOT NULL,
    responses                VARCHAR(500),
    score                    INTEGER      NOT NULL DEFAULT 0,
    severity                 VARCHAR(30),
    completed_at             TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_clinical_screenings_user_id
    ON clinical_screenings (user_id);

CREATE INDEX IF NOT EXISTS ix_clinical_screenings_user_type
    ON clinical_screenings (user_id, type);

-- ---

CREATE TABLE IF NOT EXISTS therapist_referrals (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  UUID         NOT NULL,
    reason                   VARCHAR(2000) NOT NULL,
    urgency                  VARCHAR(20)  NOT NULL,
    is_acknowledged          BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_therapist_referrals_user_id
    ON therapist_referrals (user_id);

CREATE INDEX IF NOT EXISTS ix_therapist_referrals_urgency
    ON therapist_referrals (urgency);


-- ============================================================
-- Moderation Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS content_reports (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    reporter_user_id         UUID         NOT NULL,
    content_type             VARCHAR(20)  NOT NULL,
    content_id               UUID         NOT NULL,
    reason                   VARCHAR(30)  NOT NULL,
    description              VARCHAR(4000),
    status                   VARCHAR(20)  NOT NULL DEFAULT 'Pending',
    action                   VARCHAR(30)  NOT NULL DEFAULT 'None',
    reviewed_by_user_id      UUID,
    review_notes             VARCHAR(4000),
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_at              TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_content_reports_content
    ON content_reports (content_type, content_id);

CREATE INDEX IF NOT EXISTS ix_content_reports_reporter
    ON content_reports (reporter_user_id);

CREATE INDEX IF NOT EXISTS ix_content_reports_status
    ON content_reports (status);

-- ---

CREATE TABLE IF NOT EXISTS auto_moderation_results (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    content_type             VARCHAR(20)  NOT NULL,
    content_id               UUID         NOT NULL,
    is_flagged               BOOLEAN      NOT NULL DEFAULT FALSE,
    flag_reason              VARCHAR(1000),
    confidence               DOUBLE PRECISION NOT NULL DEFAULT 0,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_auto_moderation_results_content
    ON auto_moderation_results (content_type, content_id);

CREATE INDEX IF NOT EXISTS ix_auto_moderation_results_flagged
    ON auto_moderation_results (is_flagged);


-- ============================================================
-- Learning Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS learning_paths (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title                    VARCHAR(300) NOT NULL,
    description              VARCHAR(4000),
    category                 VARCHAR(30)  NOT NULL,
    estimated_minutes        INTEGER      NOT NULL DEFAULT 0,
    module_count             INTEGER      NOT NULL DEFAULT 0,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_learning_paths_category
    ON learning_paths (category);

-- ---

CREATE TABLE IF NOT EXISTS learning_modules (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    path_id                  UUID         NOT NULL,
    title                    VARCHAR(300) NOT NULL,
    content                  TEXT         NOT NULL,
    exercise_prompt          VARCHAR(4000),
    "order"                  INTEGER      NOT NULL DEFAULT 0,
    CONSTRAINT fk_learning_modules_path
        FOREIGN KEY (path_id)
        REFERENCES learning_paths (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS ix_learning_modules_path_id
    ON learning_modules (path_id);

CREATE INDEX IF NOT EXISTS ix_learning_modules_path_order
    ON learning_modules (path_id, "order");

-- ---

CREATE TABLE IF NOT EXISTS user_learning_progress (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  UUID         NOT NULL,
    path_id                  UUID         NOT NULL,
    current_module_index     INTEGER      NOT NULL DEFAULT 0,
    completed_modules        VARCHAR(2000) DEFAULT '[]',
    reflection_notes         VARCHAR(8000) DEFAULT '{}',
    started_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at             TIMESTAMP,
    CONSTRAINT fk_user_learning_progress_path
        FOREIGN KEY (path_id)
        REFERENCES learning_paths (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS ix_user_learning_progress_user_id
    ON user_learning_progress (user_id);

CREATE INDEX IF NOT EXISTS ix_user_learning_progress_path_id
    ON user_learning_progress (path_id);

CREATE UNIQUE INDEX IF NOT EXISTS ix_user_learning_progress_user_path
    ON user_learning_progress (user_id, path_id);


-- ============================================================
-- Notification Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS device_tokens (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  UUID         NOT NULL,
    token                    VARCHAR(500) NOT NULL,
    platform                 VARCHAR(10)  NOT NULL,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS ix_device_tokens_user_token
    ON device_tokens (user_id, token);
