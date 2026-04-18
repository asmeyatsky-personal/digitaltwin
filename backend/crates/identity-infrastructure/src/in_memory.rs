//! In-memory adapters. Used by application-layer tests (§3.2 "tests use
//! in-memory adapters") and local development. Not for production.

use async_trait::async_trait;
use identity_domain::{
    DomainError,
    ports::{PasswordHasher, RepositoryError, UserRepository},
    user::User,
    values::{Email, PasswordHash},
};
use kernel::EntityId;
use std::{collections::HashMap, sync::Mutex};

#[derive(Default)]
pub struct InMemoryUserRepository {
    by_id: Mutex<HashMap<EntityId<User>, User>>,
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_id(&self, id: EntityId<User>) -> Result<Option<User>, RepositoryError> {
        Ok(self
            .by_id
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?
            .get(&id)
            .cloned())
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        let guard = self
            .by_id
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        Ok(guard
            .values()
            .find(|u| u.email().as_str() == email.as_str())
            .cloned())
    }

    async fn insert(&self, user: &User) -> Result<(), RepositoryError> {
        let mut guard = self
            .by_id
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        if guard
            .values()
            .any(|u| u.email().as_str() == user.email().as_str())
        {
            return Err(RepositoryError::Conflict("email".into()));
        }
        guard.insert(user.id(), user.clone());
        Ok(())
    }

    async fn update(&self, user: &User) -> Result<(), RepositoryError> {
        let mut guard = self
            .by_id
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        guard.insert(user.id(), user.clone());
        Ok(())
    }
}

/// Test-only hasher. Does not use a real KDF — production must use the
/// Argon2id adapter (landing with task #4 infrastructure work).
#[derive(Default)]
pub struct StubHasher;

#[async_trait]
impl PasswordHasher for StubHasher {
    async fn hash(&self, plaintext: &str) -> Result<PasswordHash, DomainError> {
        Ok(PasswordHash::from_raw(format!("stub${plaintext}")))
    }

    async fn verify(&self, plaintext: &str, hash: &PasswordHash) -> Result<bool, DomainError> {
        Ok(hash.as_str() == format!("stub${plaintext}"))
    }
}
