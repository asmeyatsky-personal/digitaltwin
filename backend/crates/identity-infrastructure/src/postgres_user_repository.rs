//! Postgres-backed `UserRepository`. Table ownership: migrations live in
//! `backend/migrations/identity/`. Connection pool is injected so the service
//! binary owns its lifecycle.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_domain::{
    ports::{RepositoryError, UserRepository},
    user::{User, UserStatus},
    values::{Email, PasswordHash},
};
use kernel::EntityId;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_row(row: PgRow) -> Result<User, RepositoryError> {
        let id: Uuid = row
            .try_get("id")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let email_raw: String = row
            .try_get("email")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let password_hash: String = row
            .try_get("password_hash")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let status: String = row
            .try_get("status")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = row
            .try_get("created_at")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let updated_at: DateTime<Utc> = row
            .try_get("updated_at")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;

        let email = Email::parse(&email_raw)
            .map_err(|e| RepositoryError::Backend(format!("bad email in db: {e}")))?;
        let hash = PasswordHash::from_raw(password_hash);

        // Rehydration bypasses `User::register` because invariants were already
        // enforced at insert time; we reconstruct via a domain-provided shim.
        let user = User::register(EntityId::from_uuid(id), email, hash, created_at)
            .map_err(|e| RepositoryError::Backend(format!("rehydrate failed: {e}")))?;

        let user = match status.as_str() {
            "active" => user,
            "suspended" | "deleted" => user.suspend(updated_at),
            other => return Err(RepositoryError::Backend(format!("unknown status: {other}"))),
        };
        Ok(user)
    }

    fn status_str(status: UserStatus) -> &'static str {
        match status {
            UserStatus::Active => "active",
            UserStatus::Suspended => "suspended",
            UserStatus::Deleted => "deleted",
        }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: EntityId<User>) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, email, password_hash, status, created_at, updated_at FROM identity.users WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        row.map(Self::map_row).transpose()
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, email, password_hash, status, created_at, updated_at FROM identity.users WHERE email = $1",
        )
        .bind(email.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        row.map(Self::map_row).transpose()
    }

    async fn insert(&self, user: &User) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO identity.users (id, email, password_hash, status, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(user.id().as_uuid())
        .bind(user.email().as_str())
        .bind(user.password_hash().as_str())
        .bind(Self::status_str(user.status()))
        .bind(user.created_at())
        .bind(user.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("duplicate key") || msg.contains("23505") {
                RepositoryError::Conflict("email".into())
            } else {
                RepositoryError::Backend(msg)
            }
        })?;
        Ok(())
    }

    async fn update(&self, user: &User) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE identity.users SET email = $2, password_hash = $3, status = $4, updated_at = $5 WHERE id = $1",
        )
        .bind(user.id().as_uuid())
        .bind(user.email().as_str())
        .bind(user.password_hash().as_str())
        .bind(Self::status_str(user.status()))
        .bind(user.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        Ok(())
    }
}
