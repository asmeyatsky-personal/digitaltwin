//! Application (Moderation bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::{Actor, AuditEvent, AuditPort, hash_state};
use kernel::{Clock, EntityId};
use moderation_domain::{
    ContentReport, DomainError, Reason, ReportRepository, Status, StoreError, UserRef,
};
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

pub struct ReportContentInput {
    pub reporter: EntityId<UserRef>,
    pub content_type: String,
    pub content_id: String,
    pub reason: Reason,
    pub actor_id: EntityId<Actor>,
}
pub struct ReportContent {
    repo: Arc<dyn ReportRepository>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}
impl ReportContent {
    pub fn new(
        repo: Arc<dyn ReportRepository>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, audit, clock }
    }
    pub async fn execute(
        &self,
        i: ReportContentInput,
    ) -> Result<EntityId<ContentReport>, UseCaseError> {
        let id = EntityId::<ContentReport>::new();
        let now = self.clock.now();
        let r = ContentReport::submit(id, i.reporter, i.content_type, i.content_id, i.reason, now)?;
        self.repo.insert(&r).await?;
        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: i.actor_id,
                action: "moderation.content.reported".into(),
                entity_type: "ContentReport".into(),
                entity_id: id.to_string(),
                before_hash: String::new(),
                after_hash: hash_state(&r),
            })
            .await?;
        Ok(id)
    }
}

pub struct ReviewReportInput {
    pub report_id: EntityId<ContentReport>,
    pub reviewer: EntityId<UserRef>,
    pub status: Status,
    pub notes: Option<String>,
    pub actor_id: EntityId<Actor>,
}
pub struct ReviewReport {
    repo: Arc<dyn ReportRepository>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}
impl ReviewReport {
    pub fn new(
        repo: Arc<dyn ReportRepository>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, audit, clock }
    }
    pub async fn execute(&self, i: ReviewReportInput) -> Result<(), UseCaseError> {
        let report = self
            .repo
            .get(i.report_id)
            .await?
            .ok_or(UseCaseError::NotFound)?;
        let now = self.clock.now();
        let reviewed = report.review(i.reviewer, i.status, i.notes, now);
        self.repo.update(&reviewed).await?;
        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: i.actor_id,
                action: "moderation.report.reviewed".into(),
                entity_type: "ContentReport".into(),
                entity_id: i.report_id.to_string(),
                before_hash: hash_state(&report),
                after_hash: hash_state(&reviewed),
            })
            .await?;
        Ok(())
    }
}

pub struct ListPending {
    repo: Arc<dyn ReportRepository>,
}
impl ListPending {
    pub fn new(repo: Arc<dyn ReportRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(&self, limit: u32) -> Result<Vec<ContentReport>, UseCaseError> {
        Ok(self.repo.list_pending(limit).await?)
    }
}
