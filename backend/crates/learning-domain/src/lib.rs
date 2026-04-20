//! Domain (Learning bounded context).

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
    #[error("title cannot be empty")]
    EmptyTitle,
    #[error("modules cannot be empty")]
    EmptyModules,
}

#[derive(Debug, Clone, Serialize)]
pub struct LearningPath {
    pub id: EntityId<LearningPath>,
    pub title: String,
    pub description: String,
    pub category: String,
    pub modules: Vec<String>,
    pub estimated_minutes: u32,
    pub created_at: DateTime<Utc>,
}
impl LearningPath {
    pub fn new(
        id: EntityId<LearningPath>,
        title: String,
        description: String,
        category: String,
        modules: Vec<String>,
        estimated_minutes: u32,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if title.trim().is_empty() {
            return Err(DomainError::EmptyTitle);
        }
        if modules.is_empty() {
            return Err(DomainError::EmptyModules);
        }
        Ok(Self {
            id,
            title,
            description,
            category,
            modules,
            estimated_minutes,
            created_at: now,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UserProgress {
    pub user_id: EntityId<UserRef>,
    pub path_id: EntityId<LearningPath>,
    pub current_module: u32,
    pub reflection_notes: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
impl UserProgress {
    pub fn start(
        user_id: EntityId<UserRef>,
        path_id: EntityId<LearningPath>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            user_id,
            path_id,
            current_module: 0,
            reflection_notes: String::new(),
            started_at: now,
            completed_at: None,
        }
    }
    pub fn advance(&self, module_count: u32, notes: String, now: DateTime<Utc>) -> Self {
        let next = self.current_module.saturating_add(1);
        let completed_at = if next >= module_count {
            Some(now)
        } else {
            None
        };
        Self {
            current_module: next,
            reflection_notes: notes,
            completed_at,
            ..self.clone()
        }
    }
}

#[async_trait::async_trait]
pub trait LearningRepository: Send + Sync {
    async fn save_path(&self, p: &LearningPath) -> Result<(), StoreError>;
    async fn list_paths(&self, category: Option<&str>) -> Result<Vec<LearningPath>, StoreError>;
    async fn get_path(
        &self,
        id: EntityId<LearningPath>,
    ) -> Result<Option<LearningPath>, StoreError>;
    async fn save_progress(&self, p: &UserProgress) -> Result<(), StoreError>;
    async fn get_progress(
        &self,
        user_id: EntityId<UserRef>,
        path_id: EntityId<LearningPath>,
    ) -> Result<Option<UserProgress>, StoreError>;
    async fn list_progress(
        &self,
        user_id: EntityId<UserRef>,
    ) -> Result<Vec<UserProgress>, StoreError>;
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
    fn rejects_empty_title() {
        assert_eq!(
            LearningPath::new(
                EntityId::new(),
                "".into(),
                "".into(),
                "c".into(),
                vec!["m1".into()],
                30,
                Utc::now()
            )
            .unwrap_err(),
            DomainError::EmptyTitle
        );
    }
    #[test]
    fn rejects_no_modules() {
        assert_eq!(
            LearningPath::new(
                EntityId::new(),
                "t".into(),
                "d".into(),
                "c".into(),
                vec![],
                30,
                Utc::now()
            )
            .unwrap_err(),
            DomainError::EmptyModules
        );
    }
    #[test]
    fn advance_marks_complete() {
        let p = UserProgress::start(EntityId::new(), EntityId::new(), Utc::now());
        let advanced = p.advance(2, "".into(), Utc::now());
        assert_eq!(advanced.current_module, 1);
        assert!(advanced.completed_at.is_none());
        let done = advanced.advance(2, "".into(), Utc::now());
        assert!(done.completed_at.is_some());
    }
}
