use crate::ports::{IssuedTokens, TokenBlacklist, TokenError, TokenIssuer};
use identity_domain::{
    DomainError,
    ports::{RepositoryError, UserRepository},
};
use kernel::Clock;
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

pub struct RefreshTokenInput {
    pub refresh_token: String,
}

pub struct RefreshTokenOutput {
    pub tokens: IssuedTokens,
}

#[derive(Debug, Error)]
pub enum RefreshTokenError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Token(#[from] TokenError),
    #[error("refresh token revoked")]
    Revoked,
}

pub struct RefreshToken {
    users: Arc<dyn UserRepository>,
    tokens: Arc<dyn TokenIssuer>,
    blacklist: Arc<dyn TokenBlacklist>,
    clock: Arc<dyn Clock>,
}

impl RefreshToken {
    #[must_use]
    pub fn new(
        users: Arc<dyn UserRepository>,
        tokens: Arc<dyn TokenIssuer>,
        blacklist: Arc<dyn TokenBlacklist>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            tokens,
            blacklist,
            clock,
        }
    }

    /// # Errors
    /// `Revoked` if the refresh JTI is blacklisted; `Token` on signature errors;
    /// `Domain` / `Repository` otherwise.
    #[instrument(skip(self, input))]
    pub async fn execute(
        &self,
        input: RefreshTokenInput,
    ) -> Result<RefreshTokenOutput, RefreshTokenError> {
        let claims = self.tokens.verify_refresh(&input.refresh_token).await?;
        if self.blacklist.is_revoked(&claims.jti).await? {
            return Err(RefreshTokenError::Revoked);
        }
        let user = self
            .users
            .find_by_id(claims.user_id)
            .await?
            .ok_or(DomainError::InvalidCredentials)?;
        user.assert_can_authenticate()?;

        // Rotate: revoke the old JTI, issue a fresh pair.
        self.blacklist
            .revoke(&claims.jti, claims.expires_at)
            .await?;
        let tokens = self.tokens.issue(&user, self.clock.now()).await?;
        Ok(RefreshTokenOutput { tokens })
    }
}
