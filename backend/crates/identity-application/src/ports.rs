//! Application-layer ports. Not in domain because tokens/blacklists are
//! auth-flow concerns that don't belong to the User aggregate invariants.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_domain::user::User;
use kernel::EntityId;
use thiserror::Error;

pub struct IssuedTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_jti: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("sign failed: {0}")]
    Sign(String),
    #[error("verify failed: {0}")]
    Verify(String),
}

#[async_trait]
pub trait TokenIssuer: Send + Sync {
    async fn issue(&self, user: &User, now: DateTime<Utc>) -> Result<IssuedTokens, TokenError>;
    async fn verify_refresh(&self, token: &str) -> Result<RefreshClaims, TokenError>;
}

#[derive(Debug, Clone)]
pub struct RefreshClaims {
    pub user_id: EntityId<User>,
    pub jti: String,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait TokenBlacklist: Send + Sync {
    async fn revoke(&self, jti: &str, expires_at: DateTime<Utc>) -> Result<(), TokenError>;
    async fn is_revoked(&self, jti: &str) -> Result<bool, TokenError>;
}
