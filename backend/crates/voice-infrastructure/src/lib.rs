//! Infrastructure (Voice). Postgres + Python voice-service proxy.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kernel::EntityId;
use reqwest::Client;
use serde_json::json;
use sqlx::{PgPool, Row, postgres::PgRow};
use std::time::Duration;
use uuid::Uuid;
use voice_domain::{
    JobStatus, StoreError, SynthError, SynthesisJob, SynthesizerPort, UserRef, VoiceProfile,
    VoiceRepository,
};

pub struct PostgresVoiceRepository {
    pool: PgPool,
}
impl PostgresVoiceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    fn row_to_profile(r: PgRow) -> Result<VoiceProfile, StoreError> {
        let user_id: Uuid = r
            .try_get("user_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let sample_url: String = r
            .try_get("sample_url")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let cloned_voice_id: Option<String> = r
            .try_get("cloned_voice_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r
            .try_get("created_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(VoiceProfile {
            user_id: EntityId::from_uuid(user_id),
            sample_url,
            cloned_voice_id,
            created_at,
        })
    }
    fn row_to_job(r: PgRow) -> Result<SynthesisJob, StoreError> {
        let id: Uuid = r
            .try_get("id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let user_id: Uuid = r
            .try_get("user_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let text: String = r
            .try_get("text")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let emotion: String = r
            .try_get("emotion")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let status: String = r
            .try_get("status")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let audio_url: Option<String> = r
            .try_get("audio_url")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r
            .try_get("created_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(SynthesisJob {
            id: EntityId::from_uuid(id),
            user_id: EntityId::from_uuid(user_id),
            text,
            emotion,
            status: JobStatus::parse(&status)
                .ok_or_else(|| StoreError::Backend("bad status".into()))?,
            audio_url,
            created_at,
        })
    }
}

#[async_trait]
impl VoiceRepository for PostgresVoiceRepository {
    async fn save_profile(&self, p: &VoiceProfile) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO voice.profiles (user_id, sample_url, cloned_voice_id, created_at) \
             VALUES ($1,$2,$3,$4) ON CONFLICT (user_id) DO UPDATE SET \
             sample_url=EXCLUDED.sample_url, cloned_voice_id=EXCLUDED.cloned_voice_id",
        )
        .bind(p.user_id.as_uuid())
        .bind(&p.sample_url)
        .bind(&p.cloned_voice_id)
        .bind(p.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn get_profile(
        &self,
        user_id: EntityId<UserRef>,
    ) -> Result<Option<VoiceProfile>, StoreError> {
        let row = sqlx::query("SELECT user_id, sample_url, cloned_voice_id, created_at FROM voice.profiles WHERE user_id=$1")
            .bind(user_id.as_uuid()).fetch_optional(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        row.map(Self::row_to_profile).transpose()
    }
    async fn save_job(&self, j: &SynthesisJob) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO voice.jobs (id, user_id, text, emotion, status, audio_url, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(j.id.as_uuid()).bind(j.user_id.as_uuid()).bind(&j.text).bind(&j.emotion)
            .bind(j.status.as_str()).bind(&j.audio_url).bind(j.created_at)
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn update_job(&self, j: &SynthesisJob) -> Result<(), StoreError> {
        sqlx::query("UPDATE voice.jobs SET status=$2, audio_url=$3 WHERE id=$1")
            .bind(j.id.as_uuid())
            .bind(j.status.as_str())
            .bind(&j.audio_url)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn get_job(
        &self,
        id: EntityId<SynthesisJob>,
    ) -> Result<Option<SynthesisJob>, StoreError> {
        let row = sqlx::query("SELECT id, user_id, text, emotion, status, audio_url, created_at FROM voice.jobs WHERE id=$1")
            .bind(id.as_uuid()).fetch_optional(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        row.map(Self::row_to_job).transpose()
    }
}

pub struct PythonVoiceAdapter {
    client: Client,
    base_url: String,
    timeout: Duration,
}
impl PythonVoiceAdapter {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest"),
            base_url: base_url.into(),
            timeout: Duration::from_secs(60),
        }
    }
}

#[async_trait]
impl SynthesizerPort for PythonVoiceAdapter {
    async fn clone_voice(
        &self,
        user_id: EntityId<UserRef>,
        sample_url: &str,
    ) -> Result<String, SynthError> {
        let url = format!("{}/clone", self.base_url);
        let payload = json!({ "user_id": user_id.to_string(), "sample_url": sample_url });
        let resp = tokio::time::timeout(self.timeout, self.client.post(&url).json(&payload).send())
            .await
            .map_err(|_| SynthError::Timeout)?
            .map_err(|e| SynthError::Failed(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(SynthError::Failed(format!("http {}", resp.status())));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SynthError::Failed(e.to_string()))?;
        body.get("voice_id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| SynthError::Failed("missing voice_id".into()))
    }
    async fn synthesize(
        &self,
        voice_id: &str,
        text: &str,
        emotion: &str,
    ) -> Result<String, SynthError> {
        let url = format!("{}/synthesize", self.base_url);
        let payload = json!({ "voice_id": voice_id, "text": text, "emotion": emotion });
        let resp = tokio::time::timeout(self.timeout, self.client.post(&url).json(&payload).send())
            .await
            .map_err(|_| SynthError::Timeout)?
            .map_err(|e| SynthError::Failed(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(SynthError::Failed(format!("http {}", resp.status())));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SynthError::Failed(e.to_string()))?;
        body.get("audio_url")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| SynthError::Failed("missing audio_url".into()))
    }
}
