//! Application (Voice bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::{Actor, AuditEvent, AuditPort, hash_state};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use thiserror::Error;
use voice_domain::{
    DomainError, StoreError, SynthError, SynthesisJob, SynthesizerPort, UserRef, VoiceProfile,
    VoiceRepository,
};

#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Synth(#[from] SynthError),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
    #[error("no voice profile for user; register first")]
    NoProfile,
    #[error("not found")]
    NotFound,
}

pub struct RegisterVoiceInput {
    pub user_id: EntityId<UserRef>,
    pub sample_url: String,
    pub actor_id: EntityId<Actor>,
}
pub struct RegisterVoice {
    repo: Arc<dyn VoiceRepository>,
    synth: Arc<dyn SynthesizerPort>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}
impl RegisterVoice {
    pub fn new(
        repo: Arc<dyn VoiceRepository>,
        synth: Arc<dyn SynthesizerPort>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            synth,
            audit,
            clock,
        }
    }
    pub async fn execute(&self, i: RegisterVoiceInput) -> Result<(), UseCaseError> {
        let now = self.clock.now();
        let profile = VoiceProfile::register(i.user_id, i.sample_url.clone(), now)?;
        let cloned = self.synth.clone_voice(i.user_id, &i.sample_url).await?;
        let with_clone = profile.with_clone(cloned);
        self.repo.save_profile(&with_clone).await?;
        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: i.actor_id,
                action: "voice.profile.registered".into(),
                entity_type: "VoiceProfile".into(),
                entity_id: i.user_id.to_string(),
                before_hash: String::new(),
                after_hash: hash_state(&with_clone),
            })
            .await?;
        Ok(())
    }
}

pub struct SynthesizeInput {
    pub user_id: EntityId<UserRef>,
    pub text: String,
    pub emotion: String,
}
pub struct SynthesizeOutput {
    pub job_id: EntityId<SynthesisJob>,
    pub audio_url: Option<String>,
}
pub struct Synthesize {
    repo: Arc<dyn VoiceRepository>,
    synth: Arc<dyn SynthesizerPort>,
    clock: Arc<dyn Clock>,
}
impl Synthesize {
    pub fn new(
        repo: Arc<dyn VoiceRepository>,
        synth: Arc<dyn SynthesizerPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, synth, clock }
    }
    pub async fn execute(&self, i: SynthesizeInput) -> Result<SynthesizeOutput, UseCaseError> {
        let profile = self
            .repo
            .get_profile(i.user_id)
            .await?
            .ok_or(UseCaseError::NoProfile)?;
        let voice_id = profile
            .cloned_voice_id
            .as_ref()
            .ok_or(UseCaseError::NoProfile)?;
        let now = self.clock.now();
        let id = EntityId::<SynthesisJob>::new();
        let job = SynthesisJob::submit(id, i.user_id, i.text.clone(), i.emotion.clone(), now)?;
        self.repo.save_job(&job).await?;
        let final_job = match self.synth.synthesize(voice_id, &i.text, &i.emotion).await {
            Ok(url) => job.complete(url),
            Err(_) => job.fail(),
        };
        self.repo.update_job(&final_job).await?;
        Ok(SynthesizeOutput {
            job_id: id,
            audio_url: final_job.audio_url,
        })
    }
}

pub struct GetJob {
    repo: Arc<dyn VoiceRepository>,
}
impl GetJob {
    pub fn new(repo: Arc<dyn VoiceRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(&self, id: EntityId<SynthesisJob>) -> Result<SynthesisJob, UseCaseError> {
        self.repo.get_job(id).await?.ok_or(UseCaseError::NotFound)
    }
}
