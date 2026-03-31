-- ============================================================
-- Digital Twin - Seed Data
-- Baseline reference data inserted on first initialization
-- No user accounts are created here; those come from registration.
-- ============================================================

-- ============================================================
-- 1. Achievement Definitions
--    Categories (enum): Emotional, Social, Growth, Consistency, Milestone
-- ============================================================

INSERT INTO achievement_definitions (id, key, title, description, icon_name, category, required_count)
VALUES
    -- Milestone achievements
    (uuid_generate_v4(), 'first_conversation',
     'First Conversation',
     'Complete your very first conversation with your AI twin.',
     'chat_bubble', 'Milestone', 1),

    (uuid_generate_v4(), 'first_journal',
     'Dear Diary',
     'Write your first journal entry.',
     'book', 'Milestone', 1),

    (uuid_generate_v4(), 'first_goal',
     'Goal Setter',
     'Create your first personal goal.',
     'flag', 'Milestone', 1),

    (uuid_generate_v4(), 'first_screening',
     'Self-Aware',
     'Complete your first clinical screening assessment.',
     'clipboard', 'Milestone', 1),

    -- Consistency achievements
    (uuid_generate_v4(), '7_day_streak',
     '7-Day Streak',
     'Check in with your AI twin for 7 consecutive days.',
     'fire', 'Consistency', 7),

    (uuid_generate_v4(), '30_day_streak',
     '30-Day Streak',
     'Check in with your AI twin for 30 consecutive days.',
     'calendar_check', 'Consistency', 30),

    (uuid_generate_v4(), 'habit_master',
     'Habit Master',
     'Complete a habit 21 times.',
     'repeat', 'Consistency', 21),

    -- Emotional achievements
    (uuid_generate_v4(), 'emotion_explorer',
     'Emotion Explorer',
     'Log 5 different emotion types across your conversations.',
     'heart', 'Emotional', 5),

    (uuid_generate_v4(), 'mood_tracker',
     'Mood Tracker',
     'Record your mood in 10 journal entries.',
     'smile', 'Emotional', 10),

    (uuid_generate_v4(), 'emotional_growth',
     'Emotional Growth',
     'Observe a positive emotional trend over 14 days.',
     'trending_up', 'Emotional', 14),

    -- Social achievements
    (uuid_generate_v4(), 'community_member',
     'Community Member',
     'Join your first community group.',
     'people', 'Social', 1),

    (uuid_generate_v4(), 'supportive_voice',
     'Supportive Voice',
     'Reply to 10 community posts.',
     'message_circle', 'Social', 10),

    (uuid_generate_v4(), 'storyteller',
     'Storyteller',
     'Contribute a chapter to a collaborative story.',
     'feather', 'Social', 1),

    -- Growth achievements
    (uuid_generate_v4(), 'lifelong_learner',
     'Lifelong Learner',
     'Complete your first learning path.',
     'graduation_cap', 'Growth', 1),

    (uuid_generate_v4(), 'creative_spirit',
     'Creative Spirit',
     'Publish 5 creative works.',
     'palette', 'Growth', 5)

ON CONFLICT (key) DO NOTHING;


-- ============================================================
-- 2. Learning Paths with Modules
--    Categories (enum): EmotionalIntelligence, Mindfulness,
--    Communication, StressManagement, Resilience, SelfCare
-- ============================================================

-- ----- Path 1: Emotional Intelligence -----
WITH path AS (
    INSERT INTO learning_paths (id, title, description, category, estimated_minutes, module_count)
    VALUES (
        uuid_generate_v4(),
        'Understanding Your Emotions',
        'Build core emotional intelligence skills. Learn to identify, understand, and manage your emotions effectively for better wellbeing and relationships.',
        'EmotionalIntelligence',
        45,
        4
    )
    RETURNING id
)
INSERT INTO learning_modules (id, path_id, title, content, exercise_prompt, "order")
SELECT uuid_generate_v4(), path.id, t.title, t.content, t.exercise_prompt, t.ord
FROM path, (VALUES
    ('Identifying Emotions',
     'Emotions are signals from your mind and body. In this module you will learn to recognise the basic emotion families -- joy, sadness, anger, fear, surprise, and calm -- and how they manifest physically.',
     'Take a moment right now. Close your eyes, breathe, and name the emotion you are feeling. Write it down along with where in your body you notice it.',
     1),
    ('Emotional Triggers',
     'Triggers are the situations, thoughts, or memories that activate an emotional response. Understanding your triggers is the first step to choosing how you respond rather than reacting automatically.',
     'Think of a recent time you had a strong emotional reaction. What was the trigger? Write the situation, the emotion, and a 1-10 intensity rating.',
     2),
    ('Emotion Regulation',
     'Regulating emotions does not mean suppressing them. It means acknowledging them and choosing a healthy response. Techniques include deep breathing, cognitive reappraisal, and grounding exercises.',
     'Try the 5-4-3-2-1 grounding technique: name 5 things you see, 4 you hear, 3 you can touch, 2 you smell, and 1 you taste. Describe how you feel afterward.',
     3),
    ('Empathy & Social Awareness',
     'Empathy is the ability to understand and share the feelings of others. It strengthens relationships and builds trust. Practice active listening and perspective-taking in everyday interactions.',
     'During your next conversation, focus entirely on listening without planning your reply. Afterward, write down what emotions you noticed in the other person.',
     4)
) AS t(title, content, exercise_prompt, ord);


-- ----- Path 2: Mindfulness -----
WITH path AS (
    INSERT INTO learning_paths (id, title, description, category, estimated_minutes, module_count)
    VALUES (
        uuid_generate_v4(),
        'Mindfulness Foundations',
        'Develop a daily mindfulness practice. Learn techniques to stay present, reduce rumination, and cultivate awareness in everyday life.',
        'Mindfulness',
        40,
        4
    )
    RETURNING id
)
INSERT INTO learning_modules (id, path_id, title, content, exercise_prompt, "order")
SELECT uuid_generate_v4(), path.id, t.title, t.content, t.exercise_prompt, t.ord
FROM path, (VALUES
    ('What Is Mindfulness?',
     'Mindfulness is the practice of paying attention to the present moment on purpose and without judgment. Research shows it reduces stress, improves focus, and supports emotional health.',
     'Set a timer for 2 minutes. Focus only on your breath -- the inhale, the pause, and the exhale. When your mind wanders, gently return. Describe the experience.',
     1),
    ('Body Scan Meditation',
     'A body scan involves directing attention slowly through each part of your body, noticing sensations without trying to change them. It builds interoceptive awareness.',
     'Lie down comfortably and scan from your toes to the top of your head, spending about 30 seconds on each area. Note any tension or warmth you discover.',
     2),
    ('Mindful Breathing',
     'Breathing is the anchor of mindfulness. By focusing on the natural rhythm of your breath, you create a point of stability you can return to any time stress arises.',
     'Practice box breathing: inhale for 4 counts, hold for 4, exhale for 4, hold for 4. Repeat 5 rounds and note how your stress level changes.',
     3),
    ('Mindfulness in Daily Life',
     'Mindfulness is not limited to meditation. You can bring awareness to eating, walking, commuting, and conversation. The goal is to move from autopilot to intentional presence.',
     'Choose one routine activity today (brushing teeth, making coffee) and do it with full attention. Write about what you noticed differently.',
     4)
) AS t(title, content, exercise_prompt, ord);


-- ----- Path 3: Communication -----
WITH path AS (
    INSERT INTO learning_paths (id, title, description, category, estimated_minutes, module_count)
    VALUES (
        uuid_generate_v4(),
        'Effective Communication',
        'Improve how you express yourself and connect with others. Covers active listening, assertiveness, non-violent communication, and giving feedback.',
        'Communication',
        50,
        4
    )
    RETURNING id
)
INSERT INTO learning_modules (id, path_id, title, content, exercise_prompt, "order")
SELECT uuid_generate_v4(), path.id, t.title, t.content, t.exercise_prompt, t.ord
FROM path, (VALUES
    ('Active Listening',
     'Active listening means fully concentrating on the speaker rather than passively hearing. Use verbal affirmations, paraphrasing, and open-ended questions to show understanding.',
     'In your next conversation, practice reflecting back what the other person said before responding with your own point. Write what you noticed about the interaction.',
     1),
    ('Assertive Expression',
     'Assertiveness is expressing your needs, opinions, and boundaries clearly and respectfully. It sits between passive (not speaking up) and aggressive (ignoring others'' needs) communication.',
     'Think of a boundary you find hard to set. Write an assertive statement using the formula: "I feel ___ when ___ because ___. I would like ___."',
     2),
    ('Non-Violent Communication',
     'NVC, developed by Marshall Rosenberg, follows four steps: observe without evaluating, state feelings, identify needs, and make clear requests. It reduces defensiveness in conversations.',
     'Recall a recent disagreement. Rewrite what you said using the four NVC steps and note how the tone changes.',
     3),
    ('Giving & Receiving Feedback',
     'Constructive feedback focuses on behaviour, not character. Use the SBI model: describe the Situation, the Behaviour, and the Impact. When receiving feedback, listen fully before responding.',
     'Think of positive feedback you have been meaning to give someone. Write it using the SBI model, then deliver it this week.',
     4)
) AS t(title, content, exercise_prompt, ord);


-- ----- Path 4: Stress Management -----
WITH path AS (
    INSERT INTO learning_paths (id, title, description, category, estimated_minutes, module_count)
    VALUES (
        uuid_generate_v4(),
        'Stress Management Toolkit',
        'Build a personalised toolkit for managing stress. Learn to identify stressors, use relaxation techniques, and create sustainable habits that protect your mental health.',
        'StressManagement',
        45,
        4
    )
    RETURNING id
)
INSERT INTO learning_modules (id, path_id, title, content, exercise_prompt, "order")
SELECT uuid_generate_v4(), path.id, t.title, t.content, t.exercise_prompt, t.ord
FROM path, (VALUES
    ('Understanding Stress',
     'Stress is a natural response to perceived threats or demands. Acute stress can be helpful, but chronic stress harms your body and mind. Recognising your stress signals is the first step.',
     'List your top 3 current stressors. For each, rate the stress from 1-10 and note whether it is within your control, partially controllable, or outside your control.',
     1),
    ('Relaxation Techniques',
     'Progressive muscle relaxation, diaphragmatic breathing, and guided imagery are evidence-based techniques that activate your parasympathetic nervous system and reduce cortisol.',
     'Try progressive muscle relaxation: tense each muscle group for 5 seconds then release. Start with your feet and work up. Describe how you feel before and after.',
     2),
    ('Time & Energy Management',
     'Poor time management is a major stressor. Prioritise tasks using the Eisenhower matrix (urgent/important), batch similar activities, and protect recovery time in your schedule.',
     'Write your to-do list for tomorrow and categorise each item as urgent/important, important/not urgent, urgent/not important, or neither. Adjust your plan based on the results.',
     3),
    ('Building Resilient Habits',
     'Resilience is built through consistent small practices: adequate sleep, regular movement, social connection, and intentional rest. These habits buffer you against future stress.',
     'Choose one resilience habit you want to start this week (e.g., 10-minute walk, screen-free evening, gratitude journaling). Commit to doing it 3 times and log the results.',
     4)
) AS t(title, content, exercise_prompt, ord);


-- ----- Path 5: Self-Care -----
WITH path AS (
    INSERT INTO learning_paths (id, title, description, category, estimated_minutes, module_count)
    VALUES (
        uuid_generate_v4(),
        'Self-Care Essentials',
        'Design a self-care practice that actually works for you. Move beyond bubble baths to address physical, emotional, social, and spiritual dimensions of wellbeing.',
        'SelfCare',
        35,
        4
    )
    RETURNING id
)
INSERT INTO learning_modules (id, path_id, title, content, exercise_prompt, "order")
SELECT uuid_generate_v4(), path.id, t.title, t.content, t.exercise_prompt, t.ord
FROM path, (VALUES
    ('What Self-Care Really Means',
     'Self-care is the intentional practice of activities that maintain or improve your health and wellbeing. It is not selfish -- it is necessary. Effective self-care spans physical, emotional, social, and intellectual dimensions.',
     'Rate your current self-care on a 1-10 scale in four areas: physical, emotional, social, and intellectual. Which area needs the most attention?',
     1),
    ('Physical Self-Care',
     'Your body is the foundation. Physical self-care includes sleep hygiene, nutrition, movement, and reducing harmful habits. Small, consistent changes outperform drastic overhauls.',
     'Track your sleep for the next 3 nights: what time you go to bed, wake up, and how rested you feel (1-10). Identify one change you could make.',
     2),
    ('Emotional & Social Self-Care',
     'Emotional self-care means processing feelings rather than avoiding them. Social self-care means nurturing relationships that energise you and setting boundaries with those that drain you.',
     'Identify 3 relationships that energise you and 1 that drains you. Write one action you will take this week to invest in a positive relationship.',
     3),
    ('Creating Your Self-Care Plan',
     'A self-care plan turns intentions into action. Schedule non-negotiable self-care time, build accountability, and review your plan monthly to adapt to changing needs.',
     'Draft a weekly self-care plan with at least one activity per dimension (physical, emotional, social, intellectual). Put the first one in your calendar now.',
     4)
) AS t(title, content, exercise_prompt, ord);


-- ============================================================
-- 3. Subscription Tiers (reference rows)
--    These are template rows; actual user subscriptions are
--    created at registration time referencing these tiers.
--    We store them in a separate lightweight table so tier
--    metadata (limits, pricing) lives in the database.
--    If a subscription_tiers table does not fit your model,
--    these serve as documentation of the three tiers.
-- ============================================================

-- Create a reference table for subscription tier metadata
CREATE TABLE IF NOT EXISTS subscription_tiers (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tier                     VARCHAR(20)  NOT NULL UNIQUE,
    display_name             VARCHAR(50)  NOT NULL,
    monthly_price_cents      INTEGER      NOT NULL DEFAULT 0,
    max_conversations_day    INTEGER      NOT NULL DEFAULT 0,    -- 0 = unlimited
    max_journal_entries_day  INTEGER      NOT NULL DEFAULT 0,
    max_goals                INTEGER      NOT NULL DEFAULT 0,
    biometric_integration    BOOLEAN      NOT NULL DEFAULT FALSE,
    therapy_access           BOOLEAN      NOT NULL DEFAULT FALSE,
    family_sharing           BOOLEAN      NOT NULL DEFAULT FALSE,
    community_access         BOOLEAN      NOT NULL DEFAULT FALSE,
    creative_tools           BOOLEAN      NOT NULL DEFAULT FALSE,
    priority_support         BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO subscription_tiers (
    id, tier, display_name, monthly_price_cents,
    max_conversations_day, max_journal_entries_day, max_goals,
    biometric_integration, therapy_access, family_sharing,
    community_access, creative_tools, priority_support
)
VALUES
    (uuid_generate_v4(), 'free', 'Free', 0,
     5, 3, 3,
     FALSE, FALSE, FALSE,
     TRUE, FALSE, FALSE),

    (uuid_generate_v4(), 'plus', 'Plus', 999,
     50, 0, 20,
     TRUE, FALSE, TRUE,
     TRUE, TRUE, FALSE),

    (uuid_generate_v4(), 'premium', 'Premium', 1999,
     0, 0, 0,
     TRUE, TRUE, TRUE,
     TRUE, TRUE, TRUE)

ON CONFLICT (tier) DO NOTHING;
