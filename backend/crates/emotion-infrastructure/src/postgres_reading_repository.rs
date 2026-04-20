use async_trait::async_trait;
use chrono::{DateTime, Utc};
use emotion_domain::{
    EmotionReading, Modality, UnifiedTone,
    ports::{ReadingRepository, RepositoryError},
    reading::{ReadingId, UserRef},
};
use kernel::EntityId;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresReadingRepository {
    pool: PgPool,
}

impl PostgresReadingRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_row(row: PgRow) -> Result<EmotionReading, RepositoryError> {
        let id: Uuid = row.try_get("id").map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let user_id: Uuid = row.try_get("user_id").map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let modality: String = row.try_get("modality").map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let tone: String = row.try_get("tone").map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let confidence: f32 = row.try_get("confidence").map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let recorded_at: DateTime<Utc> = row.try_get("recorded_at").map_err(|e| RepositoryError::Backend(e.to_string()))?;

        let modality = Modality::parse(&modality).map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let tone = UnifiedTone::parse(&tone).map_err(|e| RepositoryError::Backend(e.to_string()))?;

        EmotionReading::new(
            ReadingId::from_uuid(id),
            EntityId::<UserRef>::from_uuid(user_id),
            modality,
            tone,
            confidence,
            recorded_at,
        )
        .map_err(|e| RepositoryError::Backend(e.to_string()))
    }
}

#[async_trait]
impl ReadingRepository for PostgresReadingRepository {
    async fn insert(&self, reading: &EmotionReading) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO emotion.readings (id, user_id, modality, tone, confidence, recorded_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(reading.id().as_uuid())
        .bind(reading.user_id().as_uuid())
        .bind(reading.modality().as_str())
        .bind(reading.tone().as_str())
        .bind(reading.confidence())
        .bind(reading.recorded_at())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn list_in_window(
        &self,
        user_id: EntityId<UserRef>,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> Result<Vec<EmotionReading>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, user_id, modality, tone, confidence, recorded_at \
             FROM emotion.readings \
             WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3 \
             ORDER BY recorded_at ASC",
        )
        .bind(user_id.as_uuid())
        .bind(window_start)
        .bind(window_end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::map_row).collect()
    }

    async fn latest(
        &self,
        user_id: EntityId<UserRef>,
        limit: u32,
    ) -> Result<Vec<EmotionReading>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, user_id, modality, tone, confidence, recorded_at \
             FROM emotion.readings WHERE user_id = $1 \
             ORDER BY recorded_at DESC LIMIT $2",
        )
        .bind(user_id.as_uuid())
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::map_row).collect()
    }
}
