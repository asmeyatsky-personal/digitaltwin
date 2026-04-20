//! Application (Creative bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::{Actor, AuditEvent, AuditPort, hash_state};
use creative_domain::{
    CreativeRepository, CreativeWork, DomainError, StoreError, UserRef, WorkType,
};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
    #[error("not found")]
    NotFound,
}

pub struct CreateWorkInput {
    pub user_id: EntityId<UserRef>,
    pub work_type: WorkType,
    pub title: String,
    pub content: String,
    pub mood: String,
    pub actor_id: EntityId<Actor>,
}
pub struct CreateWork {
    repo: Arc<dyn CreativeRepository>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}
impl CreateWork {
    pub fn new(
        repo: Arc<dyn CreativeRepository>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, audit, clock }
    }
    pub async fn execute(
        &self,
        i: CreateWorkInput,
    ) -> Result<EntityId<CreativeWork>, UseCaseError> {
        let now = self.clock.now();
        let id = EntityId::<CreativeWork>::new();
        let w = CreativeWork::new(id, i.user_id, i.work_type, i.title, i.content, i.mood, now)?;
        self.repo.save(&w).await?;
        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: i.actor_id,
                action: "creative.work.created".into(),
                entity_type: "CreativeWork".into(),
                entity_id: id.to_string(),
                before_hash: String::new(),
                after_hash: hash_state(&w),
            })
            .await?;
        Ok(id)
    }
}

pub struct ShareWork {
    repo: Arc<dyn CreativeRepository>,
}
impl ShareWork {
    pub fn new(repo: Arc<dyn CreativeRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(&self, id: EntityId<CreativeWork>) -> Result<(), UseCaseError> {
        let w = self.repo.get(id).await?.ok_or(UseCaseError::NotFound)?;
        let shared = w.share();
        self.repo.update(&shared).await?;
        Ok(())
    }
}

pub struct GetWork {
    repo: Arc<dyn CreativeRepository>,
}
impl GetWork {
    pub fn new(repo: Arc<dyn CreativeRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(&self, id: EntityId<CreativeWork>) -> Result<CreativeWork, UseCaseError> {
        self.repo.get(id).await?.ok_or(UseCaseError::NotFound)
    }
}

pub struct ListWorksInput {
    pub user_id: EntityId<UserRef>,
    pub work_type: Option<WorkType>,
    pub limit: u32,
}
pub struct ListWorks {
    repo: Arc<dyn CreativeRepository>,
}
impl ListWorks {
    pub fn new(repo: Arc<dyn CreativeRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(&self, i: ListWorksInput) -> Result<Vec<CreativeWork>, UseCaseError> {
        Ok(self
            .repo
            .list_for_user(i.user_id, i.work_type, i.limit)
            .await?)
    }
}
