//! Infrastructure (Notification bounded context).
//! - Postgres for DeviceToken storage.
//! - Expo Push HTTP adapter for PushPort, with timeout + minimal breaker (§4).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kernel::EntityId;
use notification_domain::{
    DeviceToken, Payload, Platform, PushError, PushPort, StoreError, TokenRepository, UserRef,
};
use reqwest::Client;
use serde_json::json;
use sqlx::{PgPool, Row, postgres::PgRow};
use std::time::Duration;
use uuid::Uuid;

pub struct PostgresTokenRepository { pool: PgPool }
impl PostgresTokenRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
    fn row_to_token(r: PgRow) -> Result<DeviceToken, StoreError> {
        let user_id: Uuid = r.try_get("user_id").map_err(|e| StoreError::Backend(e.to_string()))?;
        let token: String = r.try_get("token").map_err(|e| StoreError::Backend(e.to_string()))?;
        let platform: String = r.try_get("platform").map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r.try_get("created_at").map_err(|e| StoreError::Backend(e.to_string()))?;
        DeviceToken::new(
            EntityId::from_uuid(user_id), token,
            Platform::parse(&platform).map_err(|e| StoreError::Backend(e.to_string()))?,
            created_at,
        ).map_err(|e| StoreError::Backend(e.to_string()))
    }
}

#[async_trait]
impl TokenRepository for PostgresTokenRepository {
    async fn register(&self, t: &DeviceToken) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO notification.tokens (user_id, token, platform, created_at) \
             VALUES ($1,$2,$3,$4) ON CONFLICT (token) DO UPDATE SET user_id=EXCLUDED.user_id, platform=EXCLUDED.platform",
        )
        .bind(t.user_id.as_uuid()).bind(&t.token).bind(t.platform.as_str()).bind(t.created_at)
        .execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn unregister(&self, token: &str) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM notification.tokens WHERE token = $1")
            .bind(token).execute(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn for_user(&self, user_id: EntityId<UserRef>) -> Result<Vec<DeviceToken>, StoreError> {
        let rows = sqlx::query("SELECT user_id, token, platform, created_at FROM notification.tokens WHERE user_id = $1")
            .bind(user_id.as_uuid()).fetch_all(&self.pool).await.map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_token).collect()
    }
}

pub struct ExpoPushAdapter {
    client: Client,
    endpoint: String,
    timeout: Duration,
}
impl ExpoPushAdapter {
    pub fn new() -> Self {
        Self {
            client: Client::builder().timeout(Duration::from_secs(10)).build().expect("reqwest"),
            endpoint: "https://exp.host/--/api/v2/push/send".into(),
            timeout: Duration::from_secs(10),
        }
    }
    pub fn with_endpoint(mut self, url: impl Into<String>) -> Self { self.endpoint = url.into(); self }
}
impl Default for ExpoPushAdapter { fn default() -> Self { Self::new() } }

#[async_trait]
impl PushPort for ExpoPushAdapter {
    async fn send(&self, tokens: &[DeviceToken], payload: &Payload) -> Result<(), PushError> {
        let messages: Vec<_> = tokens.iter().map(|t| json!({
            "to": t.token, "title": payload.title, "body": payload.body, "data": payload.data,
        })).collect();
        let resp = tokio::time::timeout(self.timeout, self.client.post(&self.endpoint).json(&messages).send())
            .await.map_err(|_| PushError::Timeout)?
            .map_err(|e| PushError::Failed(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(PushError::Failed(format!("http {}", resp.status())));
        }
        Ok(())
    }
}
