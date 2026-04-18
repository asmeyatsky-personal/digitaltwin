//! Ports consumed by `identity-application`. Implemented in
//! `identity-infrastructure`. Tests in application use in-memory adapters
//! (§3.2).

use crate::{
    errors::DomainError,
    user::User,
    values::{Email, PasswordHash},
};
use kernel::EntityId;

/// Persistence port. Implementations are async; the domain expresses the
/// contract as a trait and leaves adapter choice (Firestore, Postgres,
/// in-memory) to infrastructure.
#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: EntityId<User>) -> Result<Option<User>, RepositoryError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError>;
    async fn insert(&self, user: &User) -> Result<(), RepositoryError>;
    async fn update(&self, user: &User) -> Result<(), RepositoryError>;
}

#[async_trait::async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, plaintext: &str) -> Result<PasswordHash, DomainError>;
    async fn verify(&self, plaintext: &str, hash: &PasswordHash) -> Result<bool, DomainError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("unique constraint violated: {0}")]
    Conflict(String),
    #[error("backend error: {0}")]
    Backend(String),
}
