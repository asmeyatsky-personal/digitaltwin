//! Application (Achievement bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use achievement_domain::{
    Achievement, AchievementRepository, DomainError, StoreError, UserAchievement, UserRef,
};
use audit::{Actor, AuditEvent, AuditPort, hash_state};
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

pub struct UpsertAchievement {
    repo: Arc<dyn AchievementRepository>,
}
impl UpsertAchievement {
    pub fn new(repo: Arc<dyn AchievementRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(&self, a: Achievement) -> Result<(), UseCaseError> {
        self.repo.upsert_achievement(&a).await?;
        Ok(())
    }
}

pub struct RecordProgressInput {
    pub user_id: EntityId<UserRef>,
    pub achievement_key: String,
    pub delta: u32,
    pub actor_id: EntityId<Actor>,
}
pub struct RecordProgressOutput {
    pub progress: u32,
    pub unlocked: bool,
}
pub struct RecordProgress {
    repo: Arc<dyn AchievementRepository>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}
impl RecordProgress {
    pub fn new(
        repo: Arc<dyn AchievementRepository>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, audit, clock }
    }
    pub async fn execute(
        &self,
        i: RecordProgressInput,
    ) -> Result<RecordProgressOutput, UseCaseError> {
        let achievement = self
            .repo
            .get_by_key(&i.achievement_key)
            .await?
            .ok_or(UseCaseError::NotFound)?;
        let existing = self
            .repo
            .get_user(i.user_id, achievement.id)
            .await?
            .unwrap_or(UserAchievement {
                user_id: i.user_id,
                achievement_id: achievement.id,
                progress: 0,
                unlocked_at: None,
            });
        let now = self.clock.now();
        let was_unlocked = existing.unlocked_at.is_some();
        let updated = existing.with_progress(i.delta, achievement.required_count, now);
        self.repo.upsert_user(&updated).await?;
        if updated.unlocked_at.is_some() && !was_unlocked {
            self.audit
                .append(AuditEvent {
                    occurred_at: now,
                    actor_id: i.actor_id,
                    action: "achievement.unlocked".into(),
                    entity_type: "UserAchievement".into(),
                    entity_id: format!("{}/{}", i.user_id, achievement.id),
                    before_hash: hash_state(&existing),
                    after_hash: hash_state(&updated),
                })
                .await?;
        }
        Ok(RecordProgressOutput {
            progress: updated.progress,
            unlocked: updated.unlocked_at.is_some(),
        })
    }
}

pub struct ListAchievements {
    repo: Arc<dyn AchievementRepository>,
}
impl ListAchievements {
    pub fn new(repo: Arc<dyn AchievementRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(&self) -> Result<Vec<Achievement>, UseCaseError> {
        Ok(self.repo.list_all().await?)
    }
}

pub struct ListForUser {
    repo: Arc<dyn AchievementRepository>,
}
impl ListForUser {
    pub fn new(repo: Arc<dyn AchievementRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(
        &self,
        user_id: EntityId<UserRef>,
    ) -> Result<Vec<UserAchievement>, UseCaseError> {
        Ok(self.repo.list_for_user(user_id).await?)
    }
}
