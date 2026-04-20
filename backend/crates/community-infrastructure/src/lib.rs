//! Infrastructure (Community). Postgres.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use community_domain::{CommunityGroup, CommunityPost, CommunityRepository, StoreError, UserRef};
use kernel::EntityId;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresCommunityRepository { pool: PgPool }
impl PostgresCommunityRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
    fn row_to_group(r: PgRow) -> Result<CommunityGroup, StoreError> {
        let id: Uuid = r.try_get("id").map_err(|e| StoreError::Backend(e.to_string()))?;
        let name: String = r.try_get("name").map_err(|e| StoreError::Backend(e.to_string()))?;
        let description: String = r.try_get("description").map_err(|e| StoreError::Backend(e.to_string()))?;
        let category: String = r.try_get("category").map_err(|e| StoreError::Backend(e.to_string()))?;
        let is_moderated: bool = r.try_get("is_moderated").map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_by: Uuid = r.try_get("created_by").map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r.try_get("created_at").map_err(|e| StoreError::Backend(e.to_string()))?;
        CommunityGroup::new(EntityId::from_uuid(id), name, description, category, is_moderated, EntityId::from_uuid(created_by), created_at)
            .map_err(|e| StoreError::Backend(e.to_string()))
    }
    fn row_to_post(r: PgRow) -> Result<CommunityPost, StoreError> {
        let id: Uuid = r.try_get("id").map_err(|e| StoreError::Backend(e.to_string()))?;
        let group_id: Uuid = r.try_get("group_id").map_err(|e| StoreError::Backend(e.to_string()))?;
        let author: Uuid = r.try_get("author").map_err(|e| StoreError::Backend(e.to_string()))?;
        let content: String = r.try_get("content").map_err(|e| StoreError::Backend(e.to_string()))?;
        let is_anonymous: bool = r.try_get("is_anonymous").map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r.try_get("created_at").map_err(|e| StoreError::Backend(e.to_string()))?;
        CommunityPost::new(EntityId::from_uuid(id), EntityId::from_uuid(group_id), EntityId::from_uuid(author), content, is_anonymous, created_at)
            .map_err(|e| StoreError::Backend(e.to_string()))
    }
}

#[async_trait]
impl CommunityRepository for PostgresCommunityRepository {
    async fn create_group(&self, g: &CommunityGroup) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO community.groups (id, name, description, category, is_moderated, created_by, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(g.id.as_uuid()).bind(&g.name).bind(&g.description).bind(&g.category)
            .bind(g.is_moderated).bind(g.created_by.as_uuid()).bind(g.created_at)
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn list_groups(&self, category: Option<&str>, limit: u32) -> Result<Vec<CommunityGroup>, StoreError> {
        let rows = match category {
            Some(c) => sqlx::query("SELECT id, name, description, category, is_moderated, created_by, created_at FROM community.groups WHERE category=$1 ORDER BY created_at DESC LIMIT $2")
                .bind(c).bind(i64::from(limit)).fetch_all(&self.pool).await,
            None => sqlx::query("SELECT id, name, description, category, is_moderated, created_by, created_at FROM community.groups ORDER BY created_at DESC LIMIT $1")
                .bind(i64::from(limit)).fetch_all(&self.pool).await,
        }.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_group).collect()
    }
    async fn join(&self, group_id: EntityId<CommunityGroup>, user_id: EntityId<UserRef>) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO community.memberships (group_id, user_id, joined_at) VALUES ($1,$2,NOW()) ON CONFLICT DO NOTHING")
            .bind(group_id.as_uuid()).bind(user_id.as_uuid())
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn create_post(&self, p: &CommunityPost) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO community.posts (id, group_id, author, content, is_anonymous, created_at) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(p.id.as_uuid()).bind(p.group_id.as_uuid()).bind(p.author.as_uuid())
            .bind(&p.content).bind(p.is_anonymous).bind(p.created_at)
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn list_posts(&self, group_id: EntityId<CommunityGroup>, limit: u32) -> Result<Vec<CommunityPost>, StoreError> {
        let rows = sqlx::query("SELECT id, group_id, author, content, is_anonymous, created_at FROM community.posts WHERE group_id=$1 ORDER BY created_at DESC LIMIT $2")
            .bind(group_id.as_uuid()).bind(i64::from(limit))
            .fetch_all(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_post).collect()
    }
}
