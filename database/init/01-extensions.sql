-- ============================================================
-- Digital Twin - PostgreSQL Extensions
-- Executed on first database initialization
-- ============================================================

-- UUID generation functions (gen_random_uuid, uuid_generate_v4, etc.)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- pgvector: vector similarity search for embeddings
CREATE EXTENSION IF NOT EXISTS "vector";
