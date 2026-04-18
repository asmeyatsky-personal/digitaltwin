//! Postgres-backed refresh-token blacklist. Survives restarts (AUDIT §2.3
//! called out prior in-memory-only blacklist). Cleanup of expired rows
//! happens via a periodic background task, not at read time.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{TokenBlacklist, TokenError};
use sqlx::PgPool;

pub struct PostgresTokenBlacklist {
    pool: PgPool,
}

impl PostgresTokenBlacklist {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TokenBlacklist for PostgresTokenBlacklist {
    async fn revoke(&self, jti: &str, expires_at: DateTime<Utc>) -> Result<(), TokenError> {
        sqlx::query(
            "INSERT INTO identity.revoked_tokens (jti, expires_at) VALUES ($1, $2) \
             ON CONFLICT (jti) DO NOTHING",
        )
        .bind(jti)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| TokenError::Verify(e.to_string()))?;
        Ok(())
    }

    async fn is_revoked(&self, jti: &str) -> Result<bool, TokenError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT jti FROM identity.revoked_tokens WHERE jti = $1 AND expires_at > NOW()",
        )
        .bind(jti)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TokenError::Verify(e.to_string()))?;
        Ok(row.is_some())
    }
}
