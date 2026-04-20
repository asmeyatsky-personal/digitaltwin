//! Infrastructure (Moderation). Postgres.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kernel::EntityId;
use moderation_domain::{ContentReport, Reason, ReportRepository, Status, StoreError};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresReportRepository {
    pool: PgPool,
}
impl PostgresReportRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    fn row_to_report(r: PgRow) -> Result<ContentReport, StoreError> {
        let id: Uuid = r
            .try_get("id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let reporter: Uuid = r
            .try_get("reporter")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let content_type: String = r
            .try_get("content_type")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let content_id: String = r
            .try_get("content_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let reason: String = r
            .try_get("reason")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let status: String = r
            .try_get("status")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let reviewed_by: Option<Uuid> = r
            .try_get("reviewed_by")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let notes: Option<String> = r
            .try_get("notes")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r
            .try_get("created_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let reviewed_at: Option<DateTime<Utc>> = r
            .try_get("reviewed_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(ContentReport {
            id: EntityId::from_uuid(id),
            reporter: EntityId::from_uuid(reporter),
            content_type,
            content_id,
            reason: Reason::parse(&reason).map_err(|e| StoreError::Backend(e.to_string()))?,
            status: Status::parse(&status).map_err(|e| StoreError::Backend(e.to_string()))?,
            reviewed_by: reviewed_by.map(EntityId::from_uuid),
            notes,
            created_at,
            reviewed_at,
        })
    }
}

#[async_trait]
impl ReportRepository for PostgresReportRepository {
    async fn insert(&self, r: &ContentReport) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO moderation.reports (id, reporter, content_type, content_id, reason, status, reviewed_by, notes, created_at, reviewed_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(r.id.as_uuid()).bind(r.reporter.as_uuid()).bind(&r.content_type).bind(&r.content_id)
        .bind(r.reason.as_str()).bind(r.status.as_str())
        .bind(r.reviewed_by.map(|e| e.as_uuid())).bind(&r.notes)
        .bind(r.created_at).bind(r.reviewed_at)
        .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn update(&self, r: &ContentReport) -> Result<(), StoreError> {
        sqlx::query("UPDATE moderation.reports SET status=$2, reviewed_by=$3, notes=$4, reviewed_at=$5 WHERE id=$1")
            .bind(r.id.as_uuid()).bind(r.status.as_str())
            .bind(r.reviewed_by.map(|e| e.as_uuid())).bind(&r.notes).bind(r.reviewed_at)
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: EntityId<ContentReport>) -> Result<Option<ContentReport>, StoreError> {
        let row = sqlx::query("SELECT id, reporter, content_type, content_id, reason, status, reviewed_by, notes, created_at, reviewed_at FROM moderation.reports WHERE id=$1")
            .bind(id.as_uuid()).fetch_optional(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        row.map(Self::row_to_report).transpose()
    }
    async fn list_pending(&self, limit: u32) -> Result<Vec<ContentReport>, StoreError> {
        let rows = sqlx::query("SELECT id, reporter, content_type, content_id, reason, status, reviewed_by, notes, created_at, reviewed_at FROM moderation.reports WHERE status='pending' ORDER BY created_at ASC LIMIT $1")
            .bind(i64::from(limit)).fetch_all(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_report).collect()
    }
}
