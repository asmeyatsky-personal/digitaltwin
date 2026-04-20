use audit::{Actor, AuditEvent, AuditPort, hash_state};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use emotion_domain::{
    DomainError, EmotionReading, FusedEmotion, Modality, UnifiedTone, fuse,
    ports::{ReadingRepository, RepositoryError},
    reading::{ReadingId, UserRef},
};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

// ---- ReportReading ----

pub struct ReportReadingInput {
    pub user_id: EntityId<UserRef>,
    pub modality: Modality,
    pub tone: UnifiedTone,
    pub confidence: f32,
    pub actor_id: EntityId<Actor>,
}

pub struct ReportReadingOutput {
    pub reading_id: ReadingId,
}

#[derive(Debug, Error)]
pub enum ReportReadingError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
}

pub struct ReportReading {
    repo: Arc<dyn ReadingRepository>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}

impl ReportReading {
    #[must_use]
    pub fn new(
        repo: Arc<dyn ReadingRepository>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, audit, clock }
    }

    #[instrument(skip(self, input))]
    pub async fn execute(
        &self,
        input: ReportReadingInput,
    ) -> Result<ReportReadingOutput, ReportReadingError> {
        let now = self.clock.now();
        let id = ReadingId::new();
        let reading = EmotionReading::new(
            id,
            input.user_id,
            input.modality,
            input.tone,
            input.confidence,
            now,
        )?;
        self.repo.insert(&reading).await?;
        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: input.actor_id,
                action: "emotion.reading.reported".into(),
                entity_type: "EmotionReading".into(),
                entity_id: id.to_string(),
                before_hash: String::new(),
                after_hash: hash_state(&reading),
            })
            .await?;
        Ok(ReportReadingOutput { reading_id: id })
    }
}

// ---- FuseCurrent ----

pub struct CurrentEmotion {
    pub fused: FusedEmotion,
}

#[derive(Debug, Error)]
pub enum CurrentEmotionError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

pub struct FuseCurrent {
    repo: Arc<dyn ReadingRepository>,
    clock: Arc<dyn Clock>,
    window: ChronoDuration,
}

impl FuseCurrent {
    /// Default 5-minute window; recent-but-not-stale.
    #[must_use]
    pub fn new(repo: Arc<dyn ReadingRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            repo,
            clock,
            window: ChronoDuration::minutes(5),
        }
    }

    #[must_use]
    pub fn with_window(mut self, window: ChronoDuration) -> Self {
        self.window = window;
        self
    }

    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        user_id: EntityId<UserRef>,
    ) -> Result<CurrentEmotion, CurrentEmotionError> {
        let now = self.clock.now();
        let readings = self.repo.list_in_window(user_id, now - self.window, now).await?;
        let fused = fuse(&readings)?;
        Ok(CurrentEmotion { fused })
    }
}

// ---- GetTimeline ----

pub struct GetTimelineInput {
    pub user_id: EntityId<UserRef>,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

pub struct GetTimelineOutput {
    pub readings: Vec<EmotionReading>,
}

#[derive(Debug, Error)]
pub enum GetTimelineError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

pub struct GetTimeline {
    repo: Arc<dyn ReadingRepository>,
}

impl GetTimeline {
    #[must_use]
    pub fn new(repo: Arc<dyn ReadingRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self, input))]
    pub async fn execute(
        &self,
        input: GetTimelineInput,
    ) -> Result<GetTimelineOutput, GetTimelineError> {
        let readings = self.repo.list_in_window(input.user_id, input.from, input.to).await?;
        Ok(GetTimelineOutput { readings })
    }
}
