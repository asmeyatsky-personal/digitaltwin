//! Application (Community bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::{Actor, AuditEvent, AuditPort, hash_state};
use community_domain::{
    CommunityGroup, CommunityPost, CommunityRepository, DomainError, StoreError, UserRef,
};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error(transparent)] Domain(#[from] DomainError),
    #[error(transparent)] Store(#[from] StoreError),
    #[error(transparent)] Audit(#[from] audit::AuditError),
}

pub struct CreateGroupInput {
    pub name: String,
    pub description: String,
    pub category: String,
    pub is_moderated: bool,
    pub created_by: EntityId<UserRef>,
    pub actor_id: EntityId<Actor>,
}
pub struct CreateGroup { repo: Arc<dyn CommunityRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock> }
impl CreateGroup {
    pub fn new(repo: Arc<dyn CommunityRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock>) -> Self { Self { repo, audit, clock } }
    pub async fn execute(&self, i: CreateGroupInput) -> Result<EntityId<CommunityGroup>, UseCaseError> {
        let now = self.clock.now();
        let id = EntityId::<CommunityGroup>::new();
        let g = CommunityGroup::new(id, i.name, i.description, i.category, i.is_moderated, i.created_by, now)?;
        self.repo.create_group(&g).await?;
        self.repo.join(id, i.created_by).await?;
        self.audit.append(AuditEvent {
            occurred_at: now, actor_id: i.actor_id,
            action: "community.group.created".into(),
            entity_type: "CommunityGroup".into(), entity_id: id.to_string(),
            before_hash: String::new(), after_hash: hash_state(&g),
        }).await?;
        Ok(id)
    }
}

pub struct ListGroups { repo: Arc<dyn CommunityRepository> }
impl ListGroups {
    pub fn new(repo: Arc<dyn CommunityRepository>) -> Self { Self { repo } }
    pub async fn execute(&self, category: Option<String>, limit: u32) -> Result<Vec<CommunityGroup>, UseCaseError> {
        Ok(self.repo.list_groups(category.as_deref(), limit).await?)
    }
}

pub struct JoinGroup { repo: Arc<dyn CommunityRepository> }
impl JoinGroup {
    pub fn new(repo: Arc<dyn CommunityRepository>) -> Self { Self { repo } }
    pub async fn execute(&self, group_id: EntityId<CommunityGroup>, user_id: EntityId<UserRef>) -> Result<(), UseCaseError> {
        Ok(self.repo.join(group_id, user_id).await?)
    }
}

pub struct CreatePostInput {
    pub group_id: EntityId<CommunityGroup>,
    pub author: EntityId<UserRef>,
    pub content: String,
    pub is_anonymous: bool,
    pub actor_id: EntityId<Actor>,
}
pub struct CreatePost { repo: Arc<dyn CommunityRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock> }
impl CreatePost {
    pub fn new(repo: Arc<dyn CommunityRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock>) -> Self { Self { repo, audit, clock } }
    pub async fn execute(&self, i: CreatePostInput) -> Result<EntityId<CommunityPost>, UseCaseError> {
        let now = self.clock.now();
        let id = EntityId::<CommunityPost>::new();
        let p = CommunityPost::new(id, i.group_id, i.author, i.content, i.is_anonymous, now)?;
        self.repo.create_post(&p).await?;
        self.audit.append(AuditEvent {
            occurred_at: now, actor_id: i.actor_id,
            action: "community.post.created".into(),
            entity_type: "CommunityPost".into(), entity_id: id.to_string(),
            before_hash: String::new(), after_hash: hash_state(&p),
        }).await?;
        Ok(id)
    }
}

pub struct ListPosts { repo: Arc<dyn CommunityRepository> }
impl ListPosts {
    pub fn new(repo: Arc<dyn CommunityRepository>) -> Self { Self { repo } }
    pub async fn execute(&self, group_id: EntityId<CommunityGroup>, limit: u32) -> Result<Vec<CommunityPost>, UseCaseError> {
        Ok(self.repo.list_posts(group_id, limit).await?)
    }
}
