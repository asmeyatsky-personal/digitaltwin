//! Application (Avatar bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::{Actor, AuditEvent, AuditPort, hash_state};
use avatar_domain::{DomainError, GenerationJob, GeneratorError, GeneratorPort, JobRepository, StoreError, UserRef};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error(transparent)] Domain(#[from] DomainError),
    #[error(transparent)] Store(#[from] StoreError),
    #[error(transparent)] Generator(#[from] GeneratorError),
    #[error(transparent)] Audit(#[from] audit::AuditError),
    #[error("not found")] NotFound,
}

pub struct GenerateAvatarInput { pub user_id: EntityId<UserRef>, pub photo_url: String, pub actor_id: EntityId<Actor> }
pub struct GenerateAvatar {
    repo: Arc<dyn JobRepository>,
    generator: Arc<dyn GeneratorPort>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}
impl GenerateAvatar {
    pub fn new(repo: Arc<dyn JobRepository>, generator: Arc<dyn GeneratorPort>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, generator, audit, clock }
    }
    pub async fn execute(&self, i: GenerateAvatarInput) -> Result<EntityId<GenerationJob>, UseCaseError> {
        let now = self.clock.now();
        let id = EntityId::<GenerationJob>::new();
        let job = GenerationJob::submit(id, i.user_id, i.photo_url.clone(), now)?;
        self.repo.insert(&job).await?;
        // Synchronous generation for simplicity; swap for a queue later.
        let final_job = match self.generator.generate(id, &i.photo_url).await {
            Ok(result_url) => job.complete(result_url, self.clock.now()),
            Err(e) => job.fail(e.to_string(), self.clock.now()),
        };
        self.repo.update(&final_job).await?;
        self.audit.append(AuditEvent {
            occurred_at: now, actor_id: i.actor_id,
            action: "avatar.generation.requested".into(),
            entity_type: "GenerationJob".into(), entity_id: id.to_string(),
            before_hash: String::new(), after_hash: hash_state(&final_job),
        }).await?;
        Ok(id)
    }
}

pub struct GetJob { repo: Arc<dyn JobRepository> }
impl GetJob {
    pub fn new(repo: Arc<dyn JobRepository>) -> Self { Self { repo } }
    pub async fn execute(&self, id: EntityId<GenerationJob>) -> Result<GenerationJob, UseCaseError> {
        self.repo.get(id).await?.ok_or(UseCaseError::NotFound)
    }
}

pub struct ListJobsForUser { repo: Arc<dyn JobRepository> }
impl ListJobsForUser {
    pub fn new(repo: Arc<dyn JobRepository>) -> Self { Self { repo } }
    pub async fn execute(&self, user_id: EntityId<UserRef>) -> Result<Vec<GenerationJob>, UseCaseError> {
        Ok(self.repo.list_for_user(user_id).await?)
    }
}
