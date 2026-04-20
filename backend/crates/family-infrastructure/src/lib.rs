//! Infrastructure (Family bounded context). Postgres.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use family_domain::{Family, FamilyMember, FamilyRepository, FamilyRole, StoreError, UserRef};
use kernel::EntityId;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresFamilyRepository {
    pool: PgPool,
}
impl PostgresFamilyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    fn row_to_family(r: PgRow) -> Result<Family, StoreError> {
        let id: Uuid = r
            .try_get("id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let name: String = r
            .try_get("name")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_by: Uuid = r
            .try_get("created_by")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let created_at: DateTime<Utc> = r
            .try_get("created_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Family::new(
            EntityId::from_uuid(id),
            name,
            EntityId::from_uuid(created_by),
            created_at,
        )
        .map_err(|e| StoreError::Backend(e.to_string()))
    }
    fn row_to_member(r: PgRow) -> Result<FamilyMember, StoreError> {
        let family_id: Uuid = r
            .try_get("family_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let user_id: Uuid = r
            .try_get("user_id")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let role: String = r
            .try_get("role")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let joined_at: DateTime<Utc> = r
            .try_get("joined_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(FamilyMember {
            family_id: EntityId::from_uuid(family_id),
            user_id: EntityId::from_uuid(user_id),
            role: FamilyRole::parse(&role).map_err(|e| StoreError::Backend(e.to_string()))?,
            joined_at,
        })
    }
}

#[async_trait]
impl FamilyRepository for PostgresFamilyRepository {
    async fn insert_family(&self, f: &Family) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO family.families (id, name, created_by, created_at) VALUES ($1,$2,$3,$4)",
        )
        .bind(f.id.as_uuid())
        .bind(&f.name)
        .bind(f.created_by.as_uuid())
        .bind(f.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn get_family(&self, id: EntityId<Family>) -> Result<Option<Family>, StoreError> {
        let row =
            sqlx::query("SELECT id, name, created_by, created_at FROM family.families WHERE id=$1")
                .bind(id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StoreError::Backend(e.to_string()))?;
        row.map(Self::row_to_family).transpose()
    }
    async fn add_member(&self, m: &FamilyMember) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO family.members (family_id, user_id, role, joined_at) VALUES ($1,$2,$3,$4) \
             ON CONFLICT (family_id, user_id) DO UPDATE SET role = EXCLUDED.role",
        )
        .bind(m.family_id.as_uuid())
        .bind(m.user_id.as_uuid())
        .bind(m.role.as_str())
        .bind(m.joined_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }
    async fn list_members(
        &self,
        family_id: EntityId<Family>,
    ) -> Result<Vec<FamilyMember>, StoreError> {
        let rows = sqlx::query("SELECT family_id, user_id, role, joined_at FROM family.members WHERE family_id=$1 ORDER BY joined_at")
            .bind(family_id.as_uuid()).fetch_all(&self.pool).await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_member).collect()
    }
    async fn families_for_user(
        &self,
        user_id: EntityId<UserRef>,
    ) -> Result<Vec<Family>, StoreError> {
        let rows = sqlx::query(
            "SELECT f.id, f.name, f.created_by, f.created_at FROM family.families f \
             JOIN family.members m ON m.family_id = f.id WHERE m.user_id = $1 ORDER BY f.created_at DESC",
        )
        .bind(user_id.as_uuid()).fetch_all(&self.pool).await
        .map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::row_to_family).collect()
    }
}
