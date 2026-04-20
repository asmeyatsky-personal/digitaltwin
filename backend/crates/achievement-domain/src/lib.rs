//! Domain (Achievement bounded context).

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
    #[error("key cannot be empty")] EmptyKey,
    #[error("required_count must be > 0")] InvalidCount,
}

#[derive(Debug, Clone, Serialize)]
pub struct Achievement {
    pub id: EntityId<Achievement>,
    pub key: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub required_count: u32,
}
impl Achievement {
    pub fn new(id: EntityId<Achievement>, key: String, title: String, description: String, category: String, required_count: u32) -> Result<Self, DomainError> {
        if key.trim().is_empty() { return Err(DomainError::EmptyKey); }
        if required_count == 0 { return Err(DomainError::InvalidCount); }
        Ok(Self { id, key, title, description, category, required_count })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UserAchievement {
    pub user_id: EntityId<UserRef>,
    pub achievement_id: EntityId<Achievement>,
    pub progress: u32,
    pub unlocked_at: Option<DateTime<Utc>>,
}
impl UserAchievement {
    pub fn with_progress(&self, delta: u32, required: u32, now: DateTime<Utc>) -> Self {
        let progress = self.progress.saturating_add(delta);
        let unlocked_at = if self.unlocked_at.is_some() {
            self.unlocked_at
        } else if progress >= required {
            Some(now)
        } else {
            None
        };
        Self { progress, unlocked_at, ..self.clone() }
    }
}

#[async_trait::async_trait]
pub trait AchievementRepository: Send + Sync {
    async fn upsert_achievement(&self, a: &Achievement) -> Result<(), StoreError>;
    async fn get_by_key(&self, key: &str) -> Result<Option<Achievement>, StoreError>;
    async fn list_all(&self) -> Result<Vec<Achievement>, StoreError>;
    async fn get_user(&self, user_id: EntityId<UserRef>, achievement_id: EntityId<Achievement>) -> Result<Option<UserAchievement>, StoreError>;
    async fn upsert_user(&self, u: &UserAchievement) -> Result<(), StoreError>;
    async fn list_for_user(&self, user_id: EntityId<UserRef>) -> Result<Vec<UserAchievement>, StoreError>;
}

#[derive(Debug, Error)] pub enum StoreError { #[error("backend: {0}")] Backend(String) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rejects_empty_key() { assert_eq!(Achievement::new(EntityId::new(), "".into(), "T".into(), "D".into(), "c".into(), 1).unwrap_err(), DomainError::EmptyKey); }
    #[test] fn rejects_zero_count() { assert_eq!(Achievement::new(EntityId::new(), "k".into(), "T".into(), "D".into(), "c".into(), 0).unwrap_err(), DomainError::InvalidCount); }
    #[test] fn unlocks_on_threshold() {
        let u = UserAchievement { user_id: EntityId::new(), achievement_id: EntityId::new(), progress: 4, unlocked_at: None };
        let after = u.with_progress(1, 5, Utc::now());
        assert!(after.unlocked_at.is_some());
    }
}
