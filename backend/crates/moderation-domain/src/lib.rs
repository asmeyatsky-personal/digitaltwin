//! Domain (Moderation bounded context).

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
    #[error("content_id cannot be empty")] EmptyContentId,
    #[error("unknown reason: {0}")] UnknownReason(String),
    #[error("unknown status: {0}")] UnknownStatus(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason { Harassment, Spam, SelfHarm, Inappropriate, Other }
impl Reason {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_lowercase().as_str() {
            "harassment" => Ok(Self::Harassment), "spam" => Ok(Self::Spam),
            "self_harm"|"selfharm" => Ok(Self::SelfHarm),
            "inappropriate" => Ok(Self::Inappropriate), "other" => Ok(Self::Other),
            other => Err(DomainError::UnknownReason(other.into())),
        }
    }
    pub fn as_str(self) -> &'static str {
        match self { Self::Harassment=>"harassment",Self::Spam=>"spam",Self::SelfHarm=>"self_harm",Self::Inappropriate=>"inappropriate",Self::Other=>"other" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status { Pending, Reviewed, Actioned, Dismissed }
impl Status {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw { "pending" => Ok(Self::Pending), "reviewed" => Ok(Self::Reviewed),
                    "actioned" => Ok(Self::Actioned), "dismissed" => Ok(Self::Dismissed),
                    other => Err(DomainError::UnknownStatus(other.into())) }
    }
    pub fn as_str(self) -> &'static str {
        match self { Self::Pending=>"pending",Self::Reviewed=>"reviewed",Self::Actioned=>"actioned",Self::Dismissed=>"dismissed" }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentReport {
    pub id: EntityId<ContentReport>,
    pub reporter: EntityId<UserRef>,
    pub content_type: String,
    pub content_id: String,
    pub reason: Reason,
    pub status: Status,
    pub reviewed_by: Option<EntityId<UserRef>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}
impl ContentReport {
    pub fn submit(id: EntityId<ContentReport>, reporter: EntityId<UserRef>, content_type: String, content_id: String, reason: Reason, now: DateTime<Utc>) -> Result<Self, DomainError> {
        if content_id.trim().is_empty() { return Err(DomainError::EmptyContentId); }
        Ok(Self { id, reporter, content_type, content_id, reason, status: Status::Pending, reviewed_by: None, notes: None, created_at: now, reviewed_at: None })
    }
    pub fn review(&self, reviewer: EntityId<UserRef>, status: Status, notes: Option<String>, now: DateTime<Utc>) -> Self {
        Self { status, reviewed_by: Some(reviewer), notes, reviewed_at: Some(now), ..self.clone() }
    }
}

#[async_trait::async_trait]
pub trait ReportRepository: Send + Sync {
    async fn insert(&self, r: &ContentReport) -> Result<(), StoreError>;
    async fn update(&self, r: &ContentReport) -> Result<(), StoreError>;
    async fn get(&self, id: EntityId<ContentReport>) -> Result<Option<ContentReport>, StoreError>;
    async fn list_pending(&self, limit: u32) -> Result<Vec<ContentReport>, StoreError>;
}

#[derive(Debug, Error)] pub enum StoreError { #[error("backend: {0}")] Backend(String) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rejects_empty_content_id() {
        assert_eq!(ContentReport::submit(EntityId::new(), EntityId::new(), "post".into(), "".into(), Reason::Spam, Utc::now()).unwrap_err(), DomainError::EmptyContentId);
    }
    #[test] fn review_transitions_status() {
        let r = ContentReport::submit(EntityId::new(), EntityId::new(), "post".into(), "abc".into(), Reason::Spam, Utc::now()).unwrap();
        let reviewed = r.review(EntityId::new(), Status::Actioned, Some("blocked".into()), Utc::now());
        assert_eq!(reviewed.status, Status::Actioned);
        assert!(reviewed.reviewed_by.is_some());
    }
}
