//! Layer: domain (Memory bounded context).
//! Ports: `MemoryStore`, `LifeEventStore`.
//! Stack: Firestore for document-shaped data (ADR-0003).

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
    #[error("memory content cannot be empty")]
    EmptyContent,
    #[error("life event title cannot be empty")]
    EmptyTitle,
    #[error("unknown category: {0}")]
    UnknownCategory(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeEventCategory {
    Career,
    Relationship,
    Health,
    Education,
    Milestone,
    Loss,
    Other,
}

impl LifeEventCategory {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_lowercase().as_str() {
            "career" => Ok(Self::Career),
            "relationship" => Ok(Self::Relationship),
            "health" => Ok(Self::Health),
            "education" => Ok(Self::Education),
            "milestone" => Ok(Self::Milestone),
            "loss" => Ok(Self::Loss),
            "other" => Ok(Self::Other),
            other => Err(DomainError::UnknownCategory(other.into())),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Career => "career",
            Self::Relationship => "relationship",
            Self::Health => "health",
            Self::Education => "education",
            Self::Milestone => "milestone",
            Self::Loss => "loss",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Memory {
    pub id: EntityId<Memory>,
    pub user_id: EntityId<UserRef>,
    pub content: String,
    pub mood: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl Memory {
    pub fn new(
        id: EntityId<Memory>,
        user_id: EntityId<UserRef>,
        content: String,
        mood: String,
        tags: Vec<String>,
        created_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if content.trim().is_empty() {
            return Err(DomainError::EmptyContent);
        }
        Ok(Self { id, user_id, content, mood, tags, created_at })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LifeEvent {
    pub id: EntityId<LifeEvent>,
    pub user_id: EntityId<UserRef>,
    pub title: String,
    pub description: String,
    pub event_date: DateTime<Utc>,
    pub category: LifeEventCategory,
    pub emotional_impact: i32, // -5..=5
    pub is_recurring: bool,
}

impl LifeEvent {
    pub fn new(
        id: EntityId<LifeEvent>,
        user_id: EntityId<UserRef>,
        title: String,
        description: String,
        event_date: DateTime<Utc>,
        category: LifeEventCategory,
        emotional_impact: i32,
        is_recurring: bool,
    ) -> Result<Self, DomainError> {
        if title.trim().is_empty() {
            return Err(DomainError::EmptyTitle);
        }
        Ok(Self {
            id,
            user_id,
            title,
            description,
            event_date,
            category,
            emotional_impact: emotional_impact.clamp(-5, 5),
            is_recurring,
        })
    }
}

#[async_trait::async_trait]
pub trait MemoryStore: Send + Sync {
    async fn save(&self, memory: &Memory) -> Result<(), StoreError>;
    async fn list_for_user(
        &self,
        user_id: EntityId<UserRef>,
        limit: u32,
    ) -> Result<Vec<Memory>, StoreError>;
}

#[async_trait::async_trait]
pub trait LifeEventStore: Send + Sync {
    async fn save(&self, event: &LifeEvent) -> Result<(), StoreError>;
    async fn timeline(
        &self,
        user_id: EntityId<UserRef>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<LifeEvent>, StoreError>;
    async fn upcoming(
        &self,
        user_id: EntityId<UserRef>,
        horizon_days: u32,
    ) -> Result<Vec<LifeEvent>, StoreError>;
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
    fn memory_rejects_empty_content() {
        assert_eq!(
            Memory::new(EntityId::new(), EntityId::new(), "  ".into(), "calm".into(), vec![], Utc::now())
                .unwrap_err(),
            DomainError::EmptyContent
        );
    }

    #[test]
    fn life_event_clamps_impact() {
        let e = LifeEvent::new(
            EntityId::new(), EntityId::new(),
            "Wedding".into(), "Big day".into(),
            Utc::now(), LifeEventCategory::Milestone,
            99, false,
        ).unwrap();
        assert_eq!(e.emotional_impact, 5);
    }

    #[test]
    fn category_parse_roundtrip() {
        for v in ["career", "relationship", "health", "education", "milestone", "loss", "other"] {
            let c = LifeEventCategory::parse(v).expect(v);
            assert_eq!(c.as_str(), v);
        }
    }
}
