use crate::{
    errors::DomainError,
    values::{Email, PasswordHash},
};
use chrono::{DateTime, Utc};
use kernel::EntityId;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

/// User aggregate root. Immutable — every state transition returns a new
/// instance (§3.3). Constructor enforces invariants (§3.4).
#[derive(Debug, Clone, Serialize)]
pub struct User {
    id: EntityId<User>,
    email: Email,
    password_hash: PasswordHash,
    status: UserStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl User {
    /// Factory for new registrations. Caller must have already produced a
    /// validated `Email` and a `PasswordHash` via the `PasswordHasher` port.
    ///
    /// # Errors
    /// None today — parameters are already typed — but the signature is
    /// `Result` so future invariants (e.g. reserved domains) can be added
    /// without a breaking change.
    pub fn register(
        id: EntityId<User>,
        email: Email,
        password_hash: PasswordHash,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id,
            email,
            password_hash,
            status: UserStatus::Active,
            created_at: now,
            updated_at: now,
        })
    }

    #[must_use]
    pub fn id(&self) -> EntityId<User> {
        self.id
    }
    #[must_use]
    pub fn email(&self) -> &Email {
        &self.email
    }
    #[must_use]
    pub fn password_hash(&self) -> &PasswordHash {
        &self.password_hash
    }
    #[must_use]
    pub fn status(&self) -> UserStatus {
        self.status
    }
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// # Errors
    /// Returns `UserInactive` if the user has been suspended or deleted.
    pub fn assert_can_authenticate(&self) -> Result<(), DomainError> {
        match self.status {
            UserStatus::Active => Ok(()),
            UserStatus::Suspended | UserStatus::Deleted => Err(DomainError::UserInactive),
        }
    }

    #[must_use]
    pub fn with_new_password(&self, password_hash: PasswordHash, now: DateTime<Utc>) -> Self {
        Self {
            password_hash,
            updated_at: now,
            ..self.clone()
        }
    }

    #[must_use]
    pub fn suspend(&self, now: DateTime<Utc>) -> Self {
        Self {
            status: UserStatus::Suspended,
            updated_at: now,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> User {
        User::register(
            EntityId::new(),
            Email::parse("a@b.com").expect("valid"),
            PasswordHash::from_raw("stub".into()),
            Utc::now(),
        )
        .expect("valid")
    }

    #[test]
    fn suspend_returns_new_instance_and_keeps_original_unchanged() {
        let user = fixture();
        let suspended = user.suspend(Utc::now());
        assert_eq!(user.status(), UserStatus::Active);
        assert_eq!(suspended.status(), UserStatus::Suspended);
        assert!(suspended.updated_at() >= user.updated_at());
    }

    #[test]
    fn suspended_user_cannot_authenticate() {
        let user = fixture().suspend(Utc::now());
        assert_eq!(
            user.assert_can_authenticate(),
            Err(DomainError::UserInactive)
        );
    }

    #[test]
    fn with_new_password_does_not_mutate_original() {
        let user = fixture();
        let original_hash = user.password_hash().as_str().to_owned();
        let rotated = user.with_new_password(PasswordHash::from_raw("new".into()), Utc::now());
        assert_eq!(user.password_hash().as_str(), original_hash);
        assert_eq!(rotated.password_hash().as_str(), "new");
    }
}
