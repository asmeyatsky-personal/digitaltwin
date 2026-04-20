//! Domain (Notification bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use chrono::{DateTime, Utc};
use kernel::EntityId;
use serde::Serialize;
use thiserror::Error;

pub struct UserRef;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("token cannot be empty")]
    EmptyToken,
    #[error("unknown platform: {0}")]
    UnknownPlatform(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Ios,
    Android,
    Web,
}

impl Platform {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_lowercase().as_str() {
            "ios" => Ok(Self::Ios),
            "android" => Ok(Self::Android),
            "web" => Ok(Self::Web),
            other => Err(DomainError::UnknownPlatform(other.into())),
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
            Self::Web => "web",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceToken {
    pub user_id: EntityId<UserRef>,
    pub token: String,
    pub platform: Platform,
    pub created_at: DateTime<Utc>,
}
impl DeviceToken {
    pub fn new(
        user_id: EntityId<UserRef>,
        token: String,
        platform: Platform,
        created_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if token.trim().is_empty() {
            return Err(DomainError::EmptyToken);
        }
        Ok(Self {
            user_id,
            token,
            platform,
            created_at,
        })
    }
}

#[async_trait::async_trait]
pub trait TokenRepository: Send + Sync {
    async fn register(&self, t: &DeviceToken) -> Result<(), StoreError>;
    async fn unregister(&self, token: &str) -> Result<(), StoreError>;
    async fn for_user(&self, user_id: EntityId<UserRef>) -> Result<Vec<DeviceToken>, StoreError>;
}

pub struct Payload {
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
}

#[async_trait::async_trait]
pub trait PushPort: Send + Sync {
    async fn send(&self, tokens: &[DeviceToken], payload: &Payload) -> Result<(), PushError>;
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("backend: {0}")]
    Backend(String),
}
#[derive(Debug, Error)]
pub enum PushError {
    #[error("push: {0}")]
    Failed(String),
    #[error("timeout")]
    Timeout,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty_token() {
        assert_eq!(
            DeviceToken::new(EntityId::new(), "  ".into(), Platform::Ios, Utc::now()).unwrap_err(),
            DomainError::EmptyToken
        );
    }
    #[test]
    fn platform_roundtrip() {
        for p in ["ios", "android", "web"] {
            assert_eq!(Platform::parse(p).unwrap().as_str(), p);
        }
    }
}
