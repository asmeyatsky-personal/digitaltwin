use async_trait::async_trait;
use chrono::{DateTime, Utc};
use emotion_domain::{
    EmotionReading,
    ports::{ReadingRepository, RepositoryError},
    reading::UserRef,
};
use kernel::EntityId;
use std::sync::Mutex;

#[derive(Default)]
pub struct InMemoryReadingRepository {
    inner: Mutex<Vec<EmotionReading>>,
}

#[async_trait]
impl ReadingRepository for InMemoryReadingRepository {
    async fn insert(&self, reading: &EmotionReading) -> Result<(), RepositoryError> {
        self.inner
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?
            .push(reading.clone());
        Ok(())
    }

    async fn list_in_window(
        &self,
        user_id: EntityId<UserRef>,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> Result<Vec<EmotionReading>, RepositoryError> {
        Ok(self
            .inner
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?
            .iter()
            .filter(|r| {
                r.user_id() == user_id
                    && r.recorded_at() >= window_start
                    && r.recorded_at() <= window_end
            })
            .cloned()
            .collect())
    }

    async fn latest(
        &self,
        user_id: EntityId<UserRef>,
        limit: u32,
    ) -> Result<Vec<EmotionReading>, RepositoryError> {
        let mut matches: Vec<EmotionReading> = self
            .inner
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?
            .iter()
            .filter(|r| r.user_id() == user_id)
            .cloned()
            .collect();
        matches.sort_by_key(|r| std::cmp::Reverse(r.recorded_at()));
        matches.truncate(limit as usize);
        Ok(matches)
    }
}
