//! Domain (Avatar bounded context).

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
    #[error("photo_url cannot be empty")] EmptyPhotoUrl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus { Queued, Processing, Complete, Failed }
impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self { Self::Queued=>"queued", Self::Processing=>"processing", Self::Complete=>"complete", Self::Failed=>"failed" }
    }
    pub fn parse(raw: &str) -> Option<Self> {
        match raw { "queued"=>Some(Self::Queued),"processing"=>Some(Self::Processing),"complete"=>Some(Self::Complete),"failed"=>Some(Self::Failed),_=>None }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationJob {
    pub id: EntityId<GenerationJob>,
    pub user_id: EntityId<UserRef>,
    pub photo_url: String,
    pub status: JobStatus,
    pub result_url: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
impl GenerationJob {
    pub fn submit(id: EntityId<GenerationJob>, user_id: EntityId<UserRef>, photo_url: String, now: DateTime<Utc>) -> Result<Self, DomainError> {
        if photo_url.trim().is_empty() { return Err(DomainError::EmptyPhotoUrl); }
        Ok(Self {
            id, user_id, photo_url, status: JobStatus::Queued,
            result_url: None, failure_reason: None, created_at: now, completed_at: None,
        })
    }
    pub fn complete(&self, result_url: String, now: DateTime<Utc>) -> Self {
        Self { status: JobStatus::Complete, result_url: Some(result_url), completed_at: Some(now), ..self.clone() }
    }
    pub fn fail(&self, reason: String, now: DateTime<Utc>) -> Self {
        Self { status: JobStatus::Failed, failure_reason: Some(reason), completed_at: Some(now), ..self.clone() }
    }
}

#[async_trait::async_trait]
pub trait JobRepository: Send + Sync {
    async fn insert(&self, j: &GenerationJob) -> Result<(), StoreError>;
    async fn update(&self, j: &GenerationJob) -> Result<(), StoreError>;
    async fn get(&self, id: EntityId<GenerationJob>) -> Result<Option<GenerationJob>, StoreError>;
    async fn list_for_user(&self, user_id: EntityId<UserRef>) -> Result<Vec<GenerationJob>, StoreError>;
}

#[async_trait::async_trait]
pub trait GeneratorPort: Send + Sync {
    async fn generate(&self, job_id: EntityId<GenerationJob>, photo_url: &str) -> Result<String, GeneratorError>;
}

#[derive(Debug, Error)] pub enum StoreError { #[error("backend: {0}")] Backend(String) }
#[derive(Debug, Error)] pub enum GeneratorError { #[error("generator: {0}")] Failed(String), #[error("timeout")] Timeout }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rejects_empty_photo() {
        assert_eq!(GenerationJob::submit(EntityId::new(), EntityId::new(), "".into(), Utc::now()).unwrap_err(), DomainError::EmptyPhotoUrl);
    }
    #[test] fn complete_transitions_status() {
        let j = GenerationJob::submit(EntityId::new(), EntityId::new(), "http://x".into(), Utc::now()).unwrap();
        let done = j.complete("http://y".into(), Utc::now());
        assert_eq!(done.status, JobStatus::Complete);
        assert_eq!(done.result_url.as_deref(), Some("http://y"));
    }
}
