//! Application (Therapy bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::{Actor, AuditEvent, AuditPort, hash_state};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use therapy_domain::{
    ClinicalScreening, DomainError, ScreeningType, Severity, StoreError, TherapistProfile,
    TherapyRepository, UserRef,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error(transparent)] Domain(#[from] DomainError),
    #[error(transparent)] Store(#[from] StoreError),
    #[error(transparent)] Audit(#[from] audit::AuditError),
}

pub struct RegisterTherapistInput {
    pub name: String,
    pub credentials: String,
    pub specializations: Vec<String>,
    pub rate_per_session: u32,
    pub actor_id: EntityId<Actor>,
}
pub struct RegisterTherapist { repo: Arc<dyn TherapyRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock> }
impl RegisterTherapist {
    pub fn new(repo: Arc<dyn TherapyRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock>) -> Self { Self { repo, audit, clock } }
    pub async fn execute(&self, i: RegisterTherapistInput) -> Result<EntityId<TherapistProfile>, UseCaseError> {
        let now = self.clock.now();
        let id = EntityId::<TherapistProfile>::new();
        let t = TherapistProfile::new(id, i.name, i.credentials, i.specializations, i.rate_per_session, now)?;
        self.repo.save_therapist(&t).await?;
        self.audit.append(AuditEvent {
            occurred_at: now, actor_id: i.actor_id,
            action: "therapy.therapist.registered".into(),
            entity_type: "TherapistProfile".into(), entity_id: id.to_string(),
            before_hash: String::new(), after_hash: hash_state(&t),
        }).await?;
        Ok(id)
    }
}

pub struct ListTherapists { repo: Arc<dyn TherapyRepository> }
impl ListTherapists {
    pub fn new(repo: Arc<dyn TherapyRepository>) -> Self { Self { repo } }
    pub async fn execute(&self, limit: u32) -> Result<Vec<TherapistProfile>, UseCaseError> {
        Ok(self.repo.list_therapists(limit).await?)
    }
}

pub struct SubmitScreeningInput {
    pub user_id: EntityId<UserRef>,
    pub screening_type: ScreeningType,
    pub responses: Vec<u8>,
    pub actor_id: EntityId<Actor>,
}
pub struct SubmitScreeningOutput {
    pub screening_id: EntityId<ClinicalScreening>,
    pub score: u32,
    pub severity: Severity,
}
pub struct SubmitScreening { repo: Arc<dyn TherapyRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock> }
impl SubmitScreening {
    pub fn new(repo: Arc<dyn TherapyRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock>) -> Self { Self { repo, audit, clock } }
    pub async fn execute(&self, i: SubmitScreeningInput) -> Result<SubmitScreeningOutput, UseCaseError> {
        let now = self.clock.now();
        let id = EntityId::<ClinicalScreening>::new();
        let s = ClinicalScreening::submit(id, i.user_id, i.screening_type, i.responses, now)?;
        self.repo.save_screening(&s).await?;
        self.audit.append(AuditEvent {
            occurred_at: now, actor_id: i.actor_id,
            action: "therapy.screening.submitted".into(),
            entity_type: "ClinicalScreening".into(), entity_id: id.to_string(),
            before_hash: String::new(), after_hash: hash_state(&s),
        }).await?;
        Ok(SubmitScreeningOutput { screening_id: id, score: s.score, severity: s.severity })
    }
}

pub struct ListScreeningsForUser { repo: Arc<dyn TherapyRepository> }
impl ListScreeningsForUser {
    pub fn new(repo: Arc<dyn TherapyRepository>) -> Self { Self { repo } }
    pub async fn execute(&self, user_id: EntityId<UserRef>, limit: u32) -> Result<Vec<ClinicalScreening>, UseCaseError> {
        Ok(self.repo.screenings_for_user(user_id, limit).await?)
    }
}
