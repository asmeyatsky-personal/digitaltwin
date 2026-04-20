//! Infrastructure (Therapy). Postgres.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kernel::EntityId;
use sqlx::{PgPool, Row, postgres::PgRow};
use therapy_domain::{ClinicalScreening, ScreeningType, Severity, StoreError, TherapistProfile, TherapyRepository, UserRef};
use uuid::Uuid;

pub struct PostgresTherapyRepository { pool: PgPool }
impl PostgresTherapyRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
    fn row_to_therapist(r: PgRow) -> Result<TherapistProfile, StoreError> {
        let id: Uuid = r.try_get("id").map_err(|e| StoreError::Backend(e.to_string()))?;
        let name: String = r.try_get("name").map_err(|e| StoreError::Backend(e.to_string()))?;
        let credentials: String = r.try_get("credentials").map_err(|e| StoreError::Backend(e.to_string()))?;
        let specializations_json: String = r.try_get("specializations").map_err(|e| StoreError::Backend(e.to_string()))?;
        let rate_per_session: i32 = r.try_get("rate_per_session").map_err(|e| StoreError::Backend(e.to_string()))?;
        let is_verified: bool = r.try_get("is_verified").map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r.try_get("created_at").map_err(|e| StoreError::Backend(e.to_string()))?;
        let specializations: Vec<String> = serde_json::from_str(&specializations_json).unwrap_or_default();
        let t = TherapistProfile::new(EntityId::from_uuid(id), name, credentials, specializations, rate_per_session as u32, created_at)
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(if is_verified { t.verify() } else { t })
    }
    fn row_to_screening(r: PgRow) -> Result<ClinicalScreening, StoreError> {
        let id: Uuid = r.try_get("id").map_err(|e| StoreError::Backend(e.to_string()))?;
        let user_id: Uuid = r.try_get("user_id").map_err(|e| StoreError::Backend(e.to_string()))?;
        let screening_type: String = r.try_get("screening_type").map_err(|e| StoreError::Backend(e.to_string()))?;
        let responses_json: String = r.try_get("responses").map_err(|e| StoreError::Backend(e.to_string()))?;
        let score: i32 = r.try_get("score").map_err(|e| StoreError::Backend(e.to_string()))?;
        let severity: String = r.try_get("severity").map_err(|e| StoreError::Backend(e.to_string()))?;
        let completed_at: DateTime<Utc> = r.try_get("completed_at").map_err(|e| StoreError::Backend(e.to_string()))?;
        let responses: Vec<u8> = serde_json::from_str(&responses_json).unwrap_or_default();
        Ok(ClinicalScreening {
            id: EntityId::from_uuid(id), user_id: EntityId::from_uuid(user_id),
            screening_type: ScreeningType::parse(&screening_type).map_err(|e| StoreError::Backend(e.to_string()))?,
            responses, score: score as u32,
            severity: match severity.as_str() {
                "none" => Severity::None, "mild" => Severity::Mild,
                "moderate" => Severity::Moderate,
                "moderately_severe" => Severity::ModeratelySevere,
                "severe" => Severity::Severe,
                other => return Err(StoreError::Backend(format!("bad severity: {other}"))),
            },
            completed_at,
        })
    }
}

#[async_trait]
impl TherapyRepository for PostgresTherapyRepository {
    async fn save_therapist(&self, t: &TherapistProfile) -> Result<(), StoreError> {
        let spec = serde_json::to_string(&t.specializations).unwrap_or("[]".into());
        sqlx::query("INSERT INTO therapy.therapists (id, name, credentials, specializations, rate_per_session, is_verified, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(t.id.as_uuid()).bind(&t.name).bind(&t.credentials).bind(spec)
            .bind(t.rate_per_session as i32).bind(t.is_verified).bind(t.created_at)
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn list_therapists(&self, limit: u32) -> Result<Vec<TherapistProfile>, StoreError> {
        let rows = sqlx::query("SELECT id, name, credentials, specializations, rate_per_session, is_verified, created_at FROM therapy.therapists ORDER BY created_at DESC LIMIT $1")
            .bind(i64::from(limit)).fetch_all(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_therapist).collect()
    }
    async fn save_screening(&self, s: &ClinicalScreening) -> Result<(), StoreError> {
        let responses = serde_json::to_string(&s.responses).unwrap_or("[]".into());
        sqlx::query("INSERT INTO therapy.screenings (id, user_id, screening_type, responses, score, severity, completed_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(s.id.as_uuid()).bind(s.user_id.as_uuid()).bind(s.screening_type.as_str())
            .bind(responses).bind(s.score as i32).bind(s.severity.as_str()).bind(s.completed_at)
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn screenings_for_user(&self, user_id: EntityId<UserRef>, limit: u32) -> Result<Vec<ClinicalScreening>, StoreError> {
        let rows = sqlx::query("SELECT id, user_id, screening_type, responses, score, severity, completed_at FROM therapy.screenings WHERE user_id=$1 ORDER BY completed_at DESC LIMIT $2")
            .bind(user_id.as_uuid()).bind(i64::from(limit))
            .fetch_all(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_screening).collect()
    }
}
