//! Domain (Creative bounded context).

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
    #[error("content cannot be empty")]
    EmptyContent,
    #[error("unknown type: {0}")]
    UnknownType(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkType {
    Story,
    Poem,
    Reflection,
    Gratitude,
    Other,
}
impl WorkType {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_lowercase().as_str() {
            "story" => Ok(Self::Story),
            "poem" => Ok(Self::Poem),
            "reflection" => Ok(Self::Reflection),
            "gratitude" => Ok(Self::Gratitude),
            "other" => Ok(Self::Other),
            other => Err(DomainError::UnknownType(other.into())),
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Story => "story",
            Self::Poem => "poem",
            Self::Reflection => "reflection",
            Self::Gratitude => "gratitude",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreativeWork {
    pub id: EntityId<CreativeWork>,
    pub user_id: EntityId<UserRef>,
    pub work_type: WorkType,
    pub title: String,
    pub content: String,
    pub mood: String,
    pub is_shared: bool,
    pub created_at: DateTime<Utc>,
}
impl CreativeWork {
    pub fn new(
        id: EntityId<CreativeWork>,
        user_id: EntityId<UserRef>,
        work_type: WorkType,
        title: String,
        content: String,
        mood: String,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if title.trim().is_empty() {
            return Err(DomainError::EmptyTitle);
        }
        if content.trim().is_empty() {
            return Err(DomainError::EmptyContent);
        }
        Ok(Self {
            id,
            user_id,
            work_type,
            title,
            content,
            mood,
            is_shared: false,
            created_at: now,
        })
    }
    pub fn share(&self) -> Self {
        Self {
            is_shared: true,
            ..self.clone()
        }
    }
}

#[async_trait::async_trait]
pub trait CreativeRepository: Send + Sync {
    async fn save(&self, w: &CreativeWork) -> Result<(), StoreError>;
    async fn update(&self, w: &CreativeWork) -> Result<(), StoreError>;
    async fn get(&self, id: EntityId<CreativeWork>) -> Result<Option<CreativeWork>, StoreError>;
    async fn list_for_user(
        &self,
        user_id: EntityId<UserRef>,
        work_type: Option<WorkType>,
        limit: u32,
    ) -> Result<Vec<CreativeWork>, StoreError>;
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
            CreativeWork::new(
                EntityId::new(),
                EntityId::new(),
                WorkType::Poem,
                "".into(),
                "c".into(),
                "calm".into(),
                Utc::now()
            )
            .unwrap_err(),
            DomainError::EmptyTitle
        );
    }
    #[test]
    fn rejects_empty_content() {
        assert_eq!(
            CreativeWork::new(
                EntityId::new(),
                EntityId::new(),
                WorkType::Poem,
                "t".into(),
                "".into(),
                "calm".into(),
                Utc::now()
            )
            .unwrap_err(),
            DomainError::EmptyContent
        );
    }
    #[test]
    fn share_sets_flag() {
        let w = CreativeWork::new(
            EntityId::new(),
            EntityId::new(),
            WorkType::Story,
            "t".into(),
            "c".into(),
            "calm".into(),
            Utc::now(),
        )
        .unwrap();
        assert!(!w.is_shared);
        assert!(w.share().is_shared);
    }
}
