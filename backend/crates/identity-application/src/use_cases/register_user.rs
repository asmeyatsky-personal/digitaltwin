use audit::{AuditEvent, AuditPort, hash_state};
use identity_domain::{
    DomainError,
    ports::{PasswordHasher, RepositoryError, UserRepository},
    user::User,
    values::Email,
};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

pub struct RegisterUserInput {
    pub email: String,
    pub password: String,
    pub actor_id: EntityId<audit::Actor>,
}

pub struct RegisterUserOutput {
    pub user_id: EntityId<User>,
}

#[derive(Debug, Error)]
pub enum RegisterUserError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("email already registered")]
    EmailTaken,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
}

pub struct RegisterUser {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}

impl RegisterUser {
    #[must_use]
    pub fn new(
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            hasher,
            audit,
            clock,
        }
    }

    /// # Errors
    /// See `RegisterUserError` variants.
    #[instrument(skip(self, input), fields(email_hash))]
    pub async fn execute(
        &self,
        input: RegisterUserInput,
    ) -> Result<RegisterUserOutput, RegisterUserError> {
        let email = Email::parse(&input.email)?;
        tracing::Span::current().record(
            "email_hash",
            tracing::field::display(audit::hash_state(&email.as_str())),
        );

        if self.users.find_by_email(&email).await?.is_some() {
            return Err(RegisterUserError::EmailTaken);
        }

        let hash = self.hasher.hash(&input.password).await?;
        let now = self.clock.now();
        let id = EntityId::new();
        let user = User::register(id, email, hash, now)?;

        self.users.insert(&user).await?;

        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: input.actor_id,
                action: "identity.user.registered".into(),
                entity_type: "User".into(),
                entity_id: id.to_string(),
                before_hash: String::new(),
                after_hash: hash_state(&user),
            })
            .await?;

        Ok(RegisterUserOutput { user_id: id })
    }
}
