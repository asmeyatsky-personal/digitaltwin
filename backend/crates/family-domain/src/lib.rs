//! Domain (Family bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use chrono::{DateTime, Utc};
use kernel::EntityId;
use serde::Serialize;
use thiserror::Error;

pub struct UserRef;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("family name cannot be empty")]
    EmptyName,
    #[error("unknown role: {0}")]
    UnknownRole(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FamilyRole {
    Owner,
    Adult,
    Child,
}

impl FamilyRole {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_lowercase().as_str() {
            "owner" => Ok(Self::Owner),
            "adult" => Ok(Self::Adult),
            "child" => Ok(Self::Child),
            other => Err(DomainError::UnknownRole(other.into())),
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Adult => "adult",
            Self::Child => "child",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Family {
    pub id: EntityId<Family>,
    pub name: String,
    pub created_by: EntityId<UserRef>,
    pub created_at: DateTime<Utc>,
}
impl Family {
    pub fn new(
        id: EntityId<Family>,
        name: String,
        created_by: EntityId<UserRef>,
        created_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id,
            name,
            created_by,
            created_at,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FamilyMember {
    pub family_id: EntityId<Family>,
    pub user_id: EntityId<UserRef>,
    pub role: FamilyRole,
    pub joined_at: DateTime<Utc>,
}

#[async_trait::async_trait]
pub trait FamilyRepository: Send + Sync {
    async fn insert_family(&self, f: &Family) -> Result<(), StoreError>;
    async fn get_family(&self, id: EntityId<Family>) -> Result<Option<Family>, StoreError>;
    async fn add_member(&self, m: &FamilyMember) -> Result<(), StoreError>;
    async fn list_members(
        &self,
        family_id: EntityId<Family>,
    ) -> Result<Vec<FamilyMember>, StoreError>;
    async fn families_for_user(
        &self,
        user_id: EntityId<UserRef>,
    ) -> Result<Vec<Family>, StoreError>;
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("backend: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty_name() {
        assert_eq!(
            Family::new(EntityId::new(), "  ".into(), EntityId::new(), Utc::now()).unwrap_err(),
            DomainError::EmptyName
        );
    }
    #[test]
    fn role_parse_roundtrip() {
        for v in ["owner", "adult", "child"] {
            assert_eq!(FamilyRole::parse(v).unwrap().as_str(), v);
        }
    }
}
