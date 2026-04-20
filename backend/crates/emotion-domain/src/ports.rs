use crate::reading::{EmotionReading, UserRef};
use chrono::{DateTime, Utc};
use kernel::EntityId;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("backend error: {0}")]
    Backend(String),
}

#[async_trait::async_trait]
pub trait ReadingRepository: Send + Sync {
    async fn insert(&self, reading: &EmotionReading) -> Result<(), RepositoryError>;
    async fn list_in_window(
        &self,
        user_id: EntityId<UserRef>,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> Result<Vec<EmotionReading>, RepositoryError>;
    async fn latest(
        &self,
        user_id: EntityId<UserRef>,
        limit: u32,
    ) -> Result<Vec<EmotionReading>, RepositoryError>;
}
