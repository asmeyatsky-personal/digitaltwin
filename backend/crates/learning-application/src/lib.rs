//! Application (Learning bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::{Actor, AuditEvent, AuditPort, hash_state};
use kernel::{Clock, EntityId};
use learning_domain::{DomainError, LearningPath, LearningRepository, StoreError, UserProgress, UserRef};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error(transparent)] Domain(#[from] DomainError),
    #[error(transparent)] Store(#[from] StoreError),
    #[error(transparent)] Audit(#[from] audit::AuditError),
    #[error("not found")] NotFound,
}

pub struct CreatePathInput {
    pub title: String, pub description: String, pub category: String,
    pub modules: Vec<String>, pub estimated_minutes: u32,
}
pub struct CreatePath { repo: Arc<dyn LearningRepository>, clock: Arc<dyn Clock> }
impl CreatePath {
    pub fn new(repo: Arc<dyn LearningRepository>, clock: Arc<dyn Clock>) -> Self { Self { repo, clock } }
    pub async fn execute(&self, i: CreatePathInput) -> Result<EntityId<LearningPath>, UseCaseError> {
        let id = EntityId::<LearningPath>::new();
        let p = LearningPath::new(id, i.title, i.description, i.category, i.modules, i.estimated_minutes, self.clock.now())?;
        self.repo.save_path(&p).await?;
        Ok(id)
    }
}

pub struct ListPaths { repo: Arc<dyn LearningRepository> }
impl ListPaths {
    pub fn new(repo: Arc<dyn LearningRepository>) -> Self { Self { repo } }
    pub async fn execute(&self, category: Option<String>) -> Result<Vec<LearningPath>, UseCaseError> {
        Ok(self.repo.list_paths(category.as_deref()).await?)
    }
}

pub struct StartPathInput { pub user_id: EntityId<UserRef>, pub path_id: EntityId<LearningPath>, pub actor_id: EntityId<Actor> }
pub struct StartPath { repo: Arc<dyn LearningRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock> }
impl StartPath {
    pub fn new(repo: Arc<dyn LearningRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock>) -> Self { Self { repo, audit, clock } }
    pub async fn execute(&self, i: StartPathInput) -> Result<(), UseCaseError> {
        let now = self.clock.now();
        let p = UserProgress::start(i.user_id, i.path_id, now);
        self.repo.save_progress(&p).await?;
        self.audit.append(AuditEvent {
            occurred_at: now, actor_id: i.actor_id,
            action: "learning.path.started".into(),
            entity_type: "UserProgress".into(),
            entity_id: format!("{}/{}", i.user_id, i.path_id),
            before_hash: String::new(), after_hash: hash_state(&p),
        }).await?;
        Ok(())
    }
}

pub struct CompleteModuleInput {
    pub user_id: EntityId<UserRef>, pub path_id: EntityId<LearningPath>,
    pub reflection_notes: String, pub actor_id: EntityId<Actor>,
}
pub struct CompleteModule { repo: Arc<dyn LearningRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock> }
impl CompleteModule {
    pub fn new(repo: Arc<dyn LearningRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock>) -> Self { Self { repo, audit, clock } }
    pub async fn execute(&self, i: CompleteModuleInput) -> Result<bool, UseCaseError> {
        let path = self.repo.get_path(i.path_id).await?.ok_or(UseCaseError::NotFound)?;
        let existing = self.repo.get_progress(i.user_id, i.path_id).await?.ok_or(UseCaseError::NotFound)?;
        let now = self.clock.now();
        let module_count = u32::try_from(path.modules.len()).unwrap_or(u32::MAX);
        let advanced = existing.advance(module_count, i.reflection_notes, now);
        self.repo.save_progress(&advanced).await?;
        self.audit.append(AuditEvent {
            occurred_at: now, actor_id: i.actor_id,
            action: "learning.module.completed".into(),
            entity_type: "UserProgress".into(),
            entity_id: format!("{}/{}", i.user_id, i.path_id),
            before_hash: hash_state(&existing), after_hash: hash_state(&advanced),
        }).await?;
        Ok(advanced.completed_at.is_some())
    }
}

pub struct GetProgress { repo: Arc<dyn LearningRepository> }
impl GetProgress {
    pub fn new(repo: Arc<dyn LearningRepository>) -> Self { Self { repo } }
    pub async fn execute(&self, user_id: EntityId<UserRef>) -> Result<Vec<UserProgress>, UseCaseError> {
        Ok(self.repo.list_progress(user_id).await?)
    }
}
