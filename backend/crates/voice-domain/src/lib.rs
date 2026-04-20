//! Domain (Voice bounded context).

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
    #[error("text cannot be empty")] EmptyText,
    #[error("sample_url cannot be empty")] EmptySample,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus { Queued, Complete, Failed }
impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self { Self::Queued=>"queued", Self::Complete=>"complete", Self::Failed=>"failed" }
    }
    pub fn parse(raw: &str) -> Option<Self> {
        match raw { "queued"=>Some(Self::Queued),"complete"=>Some(Self::Complete),"failed"=>Some(Self::Failed),_=>None }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceProfile {
    pub user_id: EntityId<UserRef>,
    pub sample_url: String,
    pub cloned_voice_id: Option<String>,
    pub created_at: DateTime<Utc>,
}
impl VoiceProfile {
    pub fn register(user_id: EntityId<UserRef>, sample_url: String, now: DateTime<Utc>) -> Result<Self, DomainError> {
        if sample_url.trim().is_empty() { return Err(DomainError::EmptySample); }
        Ok(Self { user_id, sample_url, cloned_voice_id: None, created_at: now })
    }
    pub fn with_clone(&self, cloned_voice_id: String) -> Self {
        Self { cloned_voice_id: Some(cloned_voice_id), ..self.clone() }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SynthesisJob {
    pub id: EntityId<SynthesisJob>,
    pub user_id: EntityId<UserRef>,
    pub text: String,
    pub emotion: String,
    pub status: JobStatus,
    pub audio_url: Option<String>,
    pub created_at: DateTime<Utc>,
}
impl SynthesisJob {
    pub fn submit(id: EntityId<SynthesisJob>, user_id: EntityId<UserRef>, text: String, emotion: String, now: DateTime<Utc>) -> Result<Self, DomainError> {
        if text.trim().is_empty() { return Err(DomainError::EmptyText); }
        Ok(Self { id, user_id, text, emotion, status: JobStatus::Queued, audio_url: None, created_at: now })
    }
    pub fn complete(&self, audio_url: String) -> Self {
        Self { status: JobStatus::Complete, audio_url: Some(audio_url), ..self.clone() }
    }
    pub fn fail(&self) -> Self { Self { status: JobStatus::Failed, ..self.clone() } }
}

#[async_trait::async_trait]
pub trait VoiceRepository: Send + Sync {
    async fn save_profile(&self, p: &VoiceProfile) -> Result<(), StoreError>;
    async fn get_profile(&self, user_id: EntityId<UserRef>) -> Result<Option<VoiceProfile>, StoreError>;
    async fn save_job(&self, j: &SynthesisJob) -> Result<(), StoreError>;
    async fn update_job(&self, j: &SynthesisJob) -> Result<(), StoreError>;
    async fn get_job(&self, id: EntityId<SynthesisJob>) -> Result<Option<SynthesisJob>, StoreError>;
}

#[async_trait::async_trait]
pub trait SynthesizerPort: Send + Sync {
    async fn clone_voice(&self, user_id: EntityId<UserRef>, sample_url: &str) -> Result<String, SynthError>;
    async fn synthesize(&self, voice_id: &str, text: &str, emotion: &str) -> Result<String, SynthError>;
}

#[derive(Debug, Error)] pub enum StoreError { #[error("backend: {0}")] Backend(String) }
#[derive(Debug, Error)] pub enum SynthError { #[error("synthesis: {0}")] Failed(String), #[error("timeout")] Timeout }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn profile_rejects_empty_sample() {
        assert_eq!(VoiceProfile::register(EntityId::new(), "".into(), Utc::now()).unwrap_err(), DomainError::EmptySample);
    }
    #[test] fn job_rejects_empty_text() {
        assert_eq!(SynthesisJob::submit(EntityId::new(), EntityId::new(), "".into(), "neutral".into(), Utc::now()).unwrap_err(), DomainError::EmptyText);
    }
}
