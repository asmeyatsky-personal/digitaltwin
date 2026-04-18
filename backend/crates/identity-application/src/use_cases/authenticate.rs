use crate::ports::{IssuedTokens, TokenError, TokenIssuer};
use identity_domain::{
    DomainError,
    ports::{PasswordHasher, RepositoryError, UserRepository},
    values::Email,
};
use kernel::Clock;
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

pub struct AuthenticateInput {
    pub email: String,
    pub password: String,
}

pub struct AuthenticateOutput {
    pub tokens: IssuedTokens,
}

#[derive(Debug, Error)]
pub enum AuthenticateError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Token(#[from] TokenError),
}

pub struct Authenticate {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn TokenIssuer>,
    clock: Arc<dyn Clock>,
}

impl Authenticate {
    #[must_use]
    pub fn new(
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        tokens: Arc<dyn TokenIssuer>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            hasher,
            tokens,
            clock,
        }
    }

    /// # Errors
    /// `Domain(InvalidCredentials)` for unknown email or wrong password so the
    /// response shape is indistinguishable (timing-safe contract — the adapter
    /// is responsible for constant-time comparison).
    #[instrument(skip(self, input))]
    pub async fn execute(
        &self,
        input: AuthenticateInput,
    ) -> Result<AuthenticateOutput, AuthenticateError> {
        let email = Email::parse(&input.email)?;
        let user = self
            .users
            .find_by_email(&email)
            .await?
            .ok_or(DomainError::InvalidCredentials)?;
        user.assert_can_authenticate()?;
        if !self
            .hasher
            .verify(&input.password, user.password_hash())
            .await?
        {
            return Err(DomainError::InvalidCredentials.into());
        }
        let tokens = self.tokens.issue(&user, self.clock.now()).await?;
        Ok(AuthenticateOutput { tokens })
    }
}
