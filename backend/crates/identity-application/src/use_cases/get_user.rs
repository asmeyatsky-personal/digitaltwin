use chrono::{DateTime, Utc};
use identity_domain::{
    DomainError,
    ports::{RepositoryError, UserRepository},
    user::{User, UserStatus},
};
use kernel::EntityId;
use std::sync::Arc;
use thiserror::Error;

pub struct GetUserInput {
    pub user_id: EntityId<User>,
}

pub struct GetUserOutput {
    pub user_id: EntityId<User>,
    pub email: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum GetUserError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error("user not found")]
    NotFound,
}

pub struct GetUser {
    users: Arc<dyn UserRepository>,
}

impl GetUser {
    #[must_use]
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    /// # Errors
    /// `NotFound` if no user with the requested id exists.
    pub async fn execute(&self, input: GetUserInput) -> Result<GetUserOutput, GetUserError> {
        let user = self
            .users
            .find_by_id(input.user_id)
            .await?
            .ok_or(GetUserError::NotFound)?;
        Ok(GetUserOutput {
            user_id: user.id(),
            email: user.email().as_str().to_owned(),
            status: user.status(),
            created_at: user.created_at(),
        })
    }
}
