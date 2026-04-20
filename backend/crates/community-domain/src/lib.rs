//! Domain (Community bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use chrono::{DateTime, Utc};
use kernel::EntityId;
use serde::Serialize;
use thiserror::Error;

pub struct UserRef;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("name cannot be empty")]
    EmptyName,
    #[error("content cannot be empty")]
    EmptyContent,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommunityGroup {
    pub id: EntityId<CommunityGroup>,
    pub name: String,
    pub description: String,
    pub category: String,
    pub is_moderated: bool,
    pub created_by: EntityId<UserRef>,
    pub created_at: DateTime<Utc>,
}
impl CommunityGroup {
    pub fn new(
        id: EntityId<CommunityGroup>,
        name: String,
        description: String,
        category: String,
        is_moderated: bool,
        created_by: EntityId<UserRef>,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id,
            name,
            description,
            category,
            is_moderated,
            created_by,
            created_at: now,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CommunityPost {
    pub id: EntityId<CommunityPost>,
    pub group_id: EntityId<CommunityGroup>,
    pub author: EntityId<UserRef>,
    pub content: String,
    pub is_anonymous: bool,
    pub created_at: DateTime<Utc>,
}
impl CommunityPost {
    pub fn new(
        id: EntityId<CommunityPost>,
        group_id: EntityId<CommunityGroup>,
        author: EntityId<UserRef>,
        content: String,
        is_anonymous: bool,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if content.trim().is_empty() {
            return Err(DomainError::EmptyContent);
        }
        Ok(Self {
            id,
            group_id,
            author,
            content,
            is_anonymous,
            created_at: now,
        })
    }
}

#[async_trait::async_trait]
pub trait CommunityRepository: Send + Sync {
    async fn create_group(&self, g: &CommunityGroup) -> Result<(), StoreError>;
    async fn list_groups(
        &self,
        category: Option<&str>,
        limit: u32,
    ) -> Result<Vec<CommunityGroup>, StoreError>;
    async fn join(
        &self,
        group_id: EntityId<CommunityGroup>,
        user_id: EntityId<UserRef>,
    ) -> Result<(), StoreError>;
    async fn create_post(&self, p: &CommunityPost) -> Result<(), StoreError>;
    async fn list_posts(
        &self,
        group_id: EntityId<CommunityGroup>,
        limit: u32,
    ) -> Result<Vec<CommunityPost>, StoreError>;
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("backend: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty_name() {
        assert_eq!(
            CommunityGroup::new(
                EntityId::new(),
                "".into(),
                "".into(),
                "support".into(),
                false,
                EntityId::new(),
                Utc::now()
            )
            .unwrap_err(),
            DomainError::EmptyName
        );
    }
    #[test]
    fn rejects_empty_content() {
        assert_eq!(
            CommunityPost::new(
                EntityId::new(),
                EntityId::new(),
                EntityId::new(),
                "".into(),
                false,
                Utc::now()
            )
            .unwrap_err(),
            DomainError::EmptyContent
        );
    }
}
