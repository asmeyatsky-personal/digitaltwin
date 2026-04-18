use crate::ports::{TokenBlacklist, TokenError, TokenIssuer};
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

pub struct RevokeTokenInput {
    pub refresh_token: String,
}

pub struct RevokeTokenOutput {
    pub revoked: bool,
}

#[derive(Debug, Error)]
pub enum RevokeTokenError {
    #[error(transparent)]
    Token(#[from] TokenError),
}

pub struct RevokeToken {
    tokens: Arc<dyn TokenIssuer>,
    blacklist: Arc<dyn TokenBlacklist>,
}

impl RevokeToken {
    #[must_use]
    pub fn new(tokens: Arc<dyn TokenIssuer>, blacklist: Arc<dyn TokenBlacklist>) -> Self {
        Self { tokens, blacklist }
    }

    /// # Errors
    /// `Token::Verify` if the refresh token is malformed or expired.
    #[instrument(skip(self, input))]
    pub async fn execute(
        &self,
        input: RevokeTokenInput,
    ) -> Result<RevokeTokenOutput, RevokeTokenError> {
        let claims = self.tokens.verify_refresh(&input.refresh_token).await?;
        self.blacklist
            .revoke(&claims.jti, claims.expires_at)
            .await?;
        Ok(RevokeTokenOutput { revoked: true })
    }
}
