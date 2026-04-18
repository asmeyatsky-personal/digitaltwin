//! Postgres adapter for the shared `audit::AuditPort`. Writes to a schema
//! owned by a dedicated role with INSERT-only grants (§4 "separate IAM").

use async_trait::async_trait;
use audit::{AuditError, AuditEvent, AuditPort};
use sqlx::PgPool;

pub struct PostgresAuditLedger {
    pool: PgPool,
}

impl PostgresAuditLedger {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditPort for PostgresAuditLedger {
    async fn append(&self, event: AuditEvent) -> Result<(), AuditError> {
        sqlx::query(
            "INSERT INTO audit.events \
             (occurred_at, actor_id, action, entity_type, entity_id, before_hash, after_hash) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(event.occurred_at)
        .bind(event.actor_id.as_uuid())
        .bind(event.action)
        .bind(event.entity_type)
        .bind(event.entity_id)
        .bind(event.before_hash)
        .bind(event.after_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| AuditError::AppendFailed(e.to_string()))?;
        Ok(())
    }
}
