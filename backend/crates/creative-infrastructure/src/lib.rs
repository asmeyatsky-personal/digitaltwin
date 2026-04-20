//! Infrastructure (Creative). Postgres.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use creative_domain::{CreativeRepository, CreativeWork, StoreError, UserRef, WorkType};
use kernel::EntityId;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresCreativeRepository { pool: PgPool }
impl PostgresCreativeRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
    fn row_to_work(r: PgRow) -> Result<CreativeWork, StoreError> {
        let id: Uuid = r.try_get("id").map_err(|e| StoreError::Backend(e.to_string()))?;
        let user_id: Uuid = r.try_get("user_id").map_err(|e| StoreError::Backend(e.to_string()))?;
        let work_type: String = r.try_get("work_type").map_err(|e| StoreError::Backend(e.to_string()))?;
        let title: String = r.try_get("title").map_err(|e| StoreError::Backend(e.to_string()))?;
        let content: String = r.try_get("content").map_err(|e| StoreError::Backend(e.to_string()))?;
        let mood: String = r.try_get("mood").map_err(|e| StoreError::Backend(e.to_string()))?;
        let is_shared: bool = r.try_get("is_shared").map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r.try_get("created_at").map_err(|e| StoreError::Backend(e.to_string()))?;
        let w = CreativeWork::new(
            EntityId::from_uuid(id), EntityId::from_uuid(user_id),
            WorkType::parse(&work_type).map_err(|e| StoreError::Backend(e.to_string()))?,
            title, content, mood, created_at,
        ).map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(if is_shared { w.share() } else { w })
    }
}

#[async_trait]
impl CreativeRepository for PostgresCreativeRepository {
    async fn save(&self, w: &CreativeWork) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO creative.works (id, user_id, work_type, title, content, mood, is_shared, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(w.id.as_uuid()).bind(w.user_id.as_uuid()).bind(w.work_type.as_str())
            .bind(&w.title).bind(&w.content).bind(&w.mood).bind(w.is_shared).bind(w.created_at)
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn update(&self, w: &CreativeWork) -> Result<(), StoreError> {
        sqlx::query("UPDATE creative.works SET title=$2, content=$3, mood=$4, is_shared=$5 WHERE id=$1")
            .bind(w.id.as_uuid()).bind(&w.title).bind(&w.content).bind(&w.mood).bind(w.is_shared)
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: EntityId<CreativeWork>) -> Result<Option<CreativeWork>, StoreError> {
        let row = sqlx::query("SELECT id, user_id, work_type, title, content, mood, is_shared, created_at FROM creative.works WHERE id=$1")
            .bind(id.as_uuid()).fetch_optional(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        row.map(Self::row_to_work).transpose()
    }
    async fn list_for_user(&self, user_id: EntityId<UserRef>, work_type: Option<WorkType>, limit: u32) -> Result<Vec<CreativeWork>, StoreError> {
        let rows = match work_type {
            Some(t) => sqlx::query("SELECT id, user_id, work_type, title, content, mood, is_shared, created_at FROM creative.works WHERE user_id=$1 AND work_type=$2 ORDER BY created_at DESC LIMIT $3")
                .bind(user_id.as_uuid()).bind(t.as_str()).bind(i64::from(limit)).fetch_all(&self.pool).await,
            None => sqlx::query("SELECT id, user_id, work_type, title, content, mood, is_shared, created_at FROM creative.works WHERE user_id=$1 ORDER BY created_at DESC LIMIT $2")
                .bind(user_id.as_uuid()).bind(i64::from(limit)).fetch_all(&self.pool).await,
        }.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_work).collect()
    }
}
