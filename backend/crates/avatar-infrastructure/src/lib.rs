//! Infrastructure (Avatar). Postgres + Python avatar-generation-service proxy.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;
use avatar_domain::{
    GenerationJob, GeneratorError, GeneratorPort, JobRepository, JobStatus, StoreError, UserRef,
};
use chrono::{DateTime, Utc};
use kernel::EntityId;
use reqwest::Client;
use serde_json::json;
use sqlx::{PgPool, Row, postgres::PgRow};
use std::time::Duration;
use uuid::Uuid;

pub struct PostgresJobRepository {
    pool: PgPool,
}
impl PostgresJobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    fn row_to_job(r: PgRow) -> Result<GenerationJob, StoreError> {
        let id: Uuid = r
            .try_get("id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let user_id: Uuid = r
            .try_get("user_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let photo_url: String = r
            .try_get("photo_url")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let status: String = r
            .try_get("status")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let result_url: Option<String> = r
            .try_get("result_url")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let failure_reason: Option<String> = r
            .try_get("failure_reason")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r
            .try_get("created_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let completed_at: Option<DateTime<Utc>> = r
            .try_get("completed_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let status = JobStatus::parse(&status)
            .ok_or_else(|| StoreError::Backend(format!("bad status: {status}")))?;
        Ok(GenerationJob {
            id: EntityId::from_uuid(id),
            user_id: EntityId::from_uuid(user_id),
            photo_url,
            status,
            result_url,
            failure_reason,
            created_at,
            completed_at,
        })
    }
}

#[async_trait]
impl JobRepository for PostgresJobRepository {
    async fn insert(&self, j: &GenerationJob) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO avatar.jobs (id, user_id, photo_url, status, result_url, failure_reason, created_at, completed_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(j.id.as_uuid()).bind(j.user_id.as_uuid()).bind(&j.photo_url)
        .bind(j.status.as_str()).bind(&j.result_url).bind(&j.failure_reason)
        .bind(j.created_at).bind(j.completed_at)
        .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn update(&self, j: &GenerationJob) -> Result<(), StoreError> {
        sqlx::query("UPDATE avatar.jobs SET status=$2, result_url=$3, failure_reason=$4, completed_at=$5 WHERE id=$1")
            .bind(j.id.as_uuid()).bind(j.status.as_str())
            .bind(&j.result_url).bind(&j.failure_reason).bind(j.completed_at)
            .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: EntityId<GenerationJob>) -> Result<Option<GenerationJob>, StoreError> {
        let row = sqlx::query("SELECT id, user_id, photo_url, status, result_url, failure_reason, created_at, completed_at FROM avatar.jobs WHERE id=$1")
            .bind(id.as_uuid()).fetch_optional(&self.pool).await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        row.map(Self::row_to_job).transpose()
    }
    async fn list_for_user(
        &self,
        user_id: EntityId<UserRef>,
    ) -> Result<Vec<GenerationJob>, StoreError> {
        let rows = sqlx::query("SELECT id, user_id, photo_url, status, result_url, failure_reason, created_at, completed_at FROM avatar.jobs WHERE user_id=$1 ORDER BY created_at DESC")
            .bind(user_id.as_uuid()).fetch_all(&self.pool).await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_job).collect()
    }
}

pub struct PythonAvatarAdapter {
    client: Client,
    base_url: String,
    timeout: Duration,
}
impl PythonAvatarAdapter {
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
impl GeneratorPort for PythonAvatarAdapter {
    async fn generate(
        &self,
        job_id: EntityId<GenerationJob>,
        photo_url: &str,
    ) -> Result<String, GeneratorError> {
        let url = format!("{}/generate", self.base_url);
        let payload = json!({ "job_id": job_id.to_string(), "photo_url": photo_url });
        let resp = tokio::time::timeout(self.timeout, self.client.post(&url).json(&payload).send())
            .await
            .map_err(|_| GeneratorError::Timeout)?
            .map_err(|e| GeneratorError::Failed(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(GeneratorError::Failed(format!("http {}", resp.status())));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GeneratorError::Failed(e.to_string()))?;
        body.get("result_url")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| GeneratorError::Failed("missing result_url".into()))
    }
}
