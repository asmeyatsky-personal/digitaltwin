//! Infrastructure (Achievement). Postgres.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use achievement_domain::{
    Achievement, AchievementRepository, StoreError, UserAchievement, UserRef,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kernel::EntityId;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresAchievementRepository {
    pool: PgPool,
}
impl PostgresAchievementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    fn row_to_achievement(r: PgRow) -> Result<Achievement, StoreError> {
        let id: Uuid = r
            .try_get("id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let key: String = r
            .try_get("key")
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
        let required_count: i32 = r
            .try_get("required_count")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Achievement::new(
            EntityId::from_uuid(id),
            key,
            title,
            description,
            category,
            required_count as u32,
        )
        .map_err(|e| StoreError::Backend(e.to_string()))
    }
    fn row_to_user(r: PgRow) -> Result<UserAchievement, StoreError> {
        let user_id: Uuid = r
            .try_get("user_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let achievement_id: Uuid = r
            .try_get("achievement_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let progress: i32 = r
            .try_get("progress")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let unlocked_at: Option<DateTime<Utc>> = r
            .try_get("unlocked_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(UserAchievement {
            user_id: EntityId::from_uuid(user_id),
            achievement_id: EntityId::from_uuid(achievement_id),
            progress: progress as u32,
            unlocked_at,
        })
    }
}

#[async_trait]
impl AchievementRepository for PostgresAchievementRepository {
    async fn upsert_achievement(&self, a: &Achievement) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO achievement.achievements (id, key, title, description, category, required_count) \
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (key) DO UPDATE SET \
             title = EXCLUDED.title, description = EXCLUDED.description, category = EXCLUDED.category, required_count = EXCLUDED.required_count",
        )
        .bind(a.id.as_uuid()).bind(&a.key).bind(&a.title).bind(&a.description)
        .bind(&a.category).bind(a.required_count as i32)
        .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn get_by_key(&self, key: &str) -> Result<Option<Achievement>, StoreError> {
        let row = sqlx::query("SELECT id, key, title, description, category, required_count FROM achievement.achievements WHERE key = $1")
            .bind(key).fetch_optional(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        row.map(Self::row_to_achievement).transpose()
    }
    async fn list_all(&self) -> Result<Vec<Achievement>, StoreError> {
        let rows = sqlx::query("SELECT id, key, title, description, category, required_count FROM achievement.achievements ORDER BY category, key")
            .fetch_all(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_achievement).collect()
    }
    async fn get_user(
        &self,
        user_id: EntityId<UserRef>,
        achievement_id: EntityId<Achievement>,
    ) -> Result<Option<UserAchievement>, StoreError> {
        let row = sqlx::query("SELECT user_id, achievement_id, progress, unlocked_at FROM achievement.user_achievements WHERE user_id = $1 AND achievement_id = $2")
            .bind(user_id.as_uuid()).bind(achievement_id.as_uuid())
            .fetch_optional(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        row.map(Self::row_to_user).transpose()
    }
    async fn upsert_user(&self, u: &UserAchievement) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO achievement.user_achievements (user_id, achievement_id, progress, unlocked_at) \
             VALUES ($1,$2,$3,$4) ON CONFLICT (user_id, achievement_id) DO UPDATE SET \
             progress = EXCLUDED.progress, unlocked_at = COALESCE(achievement.user_achievements.unlocked_at, EXCLUDED.unlocked_at)",
        )
        .bind(u.user_id.as_uuid()).bind(u.achievement_id.as_uuid())
        .bind(u.progress as i32).bind(u.unlocked_at)
        .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn list_for_user(
        &self,
        user_id: EntityId<UserRef>,
    ) -> Result<Vec<UserAchievement>, StoreError> {
        let rows = sqlx::query("SELECT user_id, achievement_id, progress, unlocked_at FROM achievement.user_achievements WHERE user_id = $1")
            .bind(user_id.as_uuid()).fetch_all(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_user).collect()
    }
}
