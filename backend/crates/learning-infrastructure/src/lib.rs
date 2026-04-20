//! Infrastructure (Learning). Postgres.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kernel::EntityId;
use learning_domain::{LearningPath, LearningRepository, StoreError, UserProgress, UserRef};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresLearningRepository {
    pool: PgPool,
}
impl PostgresLearningRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    fn row_to_path(r: PgRow) -> Result<LearningPath, StoreError> {
        let id: Uuid = r
            .try_get("id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let title: String = r
            .try_get("title")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let description: String = r
            .try_get("description")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let category: String = r
            .try_get("category")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let modules_json: String = r
            .try_get("modules")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let estimated_minutes: i32 = r
            .try_get("estimated_minutes")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r
            .try_get("created_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let modules: Vec<String> = serde_json::from_str(&modules_json).unwrap_or_default();
        LearningPath::new(
            EntityId::from_uuid(id),
            title,
            description,
            category,
            modules,
            estimated_minutes as u32,
            created_at,
        )
        .map_err(|e| StoreError::Backend(e.to_string()))
    }
    fn row_to_progress(r: PgRow) -> Result<UserProgress, StoreError> {
        let user_id: Uuid = r
            .try_get("user_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let path_id: Uuid = r
            .try_get("path_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let current_module: i32 = r
            .try_get("current_module")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let reflection_notes: String = r
            .try_get("reflection_notes")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let started_at: DateTime<Utc> = r
            .try_get("started_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let completed_at: Option<DateTime<Utc>> = r
            .try_get("completed_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(UserProgress {
            user_id: EntityId::from_uuid(user_id),
            path_id: EntityId::from_uuid(path_id),
            current_module: current_module as u32,
            reflection_notes,
            started_at,
            completed_at,
        })
    }
}

#[async_trait]
impl LearningRepository for PostgresLearningRepository {
    async fn save_path(&self, p: &LearningPath) -> Result<(), StoreError> {
        let modules = serde_json::to_string(&p.modules).unwrap_or("[]".into());
        sqlx::query("INSERT INTO learning.paths (id, title, description, category, modules, estimated_minutes, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(p.id.as_uuid()).bind(&p.title).bind(&p.description).bind(&p.category)
            .bind(modules).bind(p.estimated_minutes as i32).bind(p.created_at)
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn list_paths(&self, category: Option<&str>) -> Result<Vec<LearningPath>, StoreError> {
        let rows = match category {
            Some(c) => sqlx::query("SELECT id, title, description, category, modules, estimated_minutes, created_at FROM learning.paths WHERE category=$1")
                .bind(c).fetch_all(&self.pool).await,
            None => sqlx::query("SELECT id, title, description, category, modules, estimated_minutes, created_at FROM learning.paths")
                .fetch_all(&self.pool).await,
        }.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_path).collect()
    }
    async fn get_path(
        &self,
        id: EntityId<LearningPath>,
    ) -> Result<Option<LearningPath>, StoreError> {
        let row = sqlx::query("SELECT id, title, description, category, modules, estimated_minutes, created_at FROM learning.paths WHERE id=$1")
            .bind(id.as_uuid()).fetch_optional(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        row.map(Self::row_to_path).transpose()
    }
    async fn save_progress(&self, p: &UserProgress) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO learning.progress (user_id, path_id, current_module, reflection_notes, started_at, completed_at) \
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (user_id, path_id) DO UPDATE SET \
             current_module=EXCLUDED.current_module, reflection_notes=EXCLUDED.reflection_notes, completed_at=EXCLUDED.completed_at",
        )
        .bind(p.user_id.as_uuid()).bind(p.path_id.as_uuid())
        .bind(p.current_module as i32).bind(&p.reflection_notes)
        .bind(p.started_at).bind(p.completed_at)
        .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn get_progress(
        &self,
        user_id: EntityId<UserRef>,
        path_id: EntityId<LearningPath>,
    ) -> Result<Option<UserProgress>, StoreError> {
        let row = sqlx::query("SELECT user_id, path_id, current_module, reflection_notes, started_at, completed_at FROM learning.progress WHERE user_id=$1 AND path_id=$2")
            .bind(user_id.as_uuid()).bind(path_id.as_uuid())
            .fetch_optional(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        row.map(Self::row_to_progress).transpose()
    }
    async fn list_progress(
        &self,
        user_id: EntityId<UserRef>,
    ) -> Result<Vec<UserProgress>, StoreError> {
        let rows = sqlx::query("SELECT user_id, path_id, current_module, reflection_notes, started_at, completed_at FROM learning.progress WHERE user_id=$1")
            .bind(user_id.as_uuid()).fetch_all(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_progress).collect()
    }
}
