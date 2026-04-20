//! Application (Family bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::{Actor, AuditEvent, AuditPort, hash_state};
use family_domain::{
    DomainError, Family, FamilyMember, FamilyRepository, FamilyRole, StoreError, UserRef,
};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
    #[error("not found")]
    NotFound,
}

pub struct CreateFamilyInput {
    pub name: String,
    pub created_by: EntityId<UserRef>,
    pub actor_id: EntityId<Actor>,
}
pub struct CreateFamily {
    repo: Arc<dyn FamilyRepository>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}
impl CreateFamily {
    pub fn new(
        repo: Arc<dyn FamilyRepository>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, audit, clock }
    }
    pub async fn execute(&self, i: CreateFamilyInput) -> Result<EntityId<Family>, UseCaseError> {
        let id = EntityId::<Family>::new();
        let now = self.clock.now();
        let f = Family::new(id, i.name, i.created_by, now)?;
        self.repo.insert_family(&f).await?;
        // Creator is the owner.
        self.repo
            .add_member(&FamilyMember {
                family_id: id,
                user_id: i.created_by,
                role: FamilyRole::Owner,
                joined_at: now,
            })
            .await?;
        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: i.actor_id,
                action: "family.created".into(),
                entity_type: "Family".into(),
                entity_id: id.to_string(),
                before_hash: String::new(),
                after_hash: hash_state(&f),
            })
            .await?;
        Ok(id)
    }
}

pub struct AddMemberInput {
    pub family_id: EntityId<Family>,
    pub user_id: EntityId<UserRef>,
    pub role: FamilyRole,
    pub actor_id: EntityId<Actor>,
}
pub struct AddMember {
    repo: Arc<dyn FamilyRepository>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}
impl AddMember {
    pub fn new(
        repo: Arc<dyn FamilyRepository>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, audit, clock }
    }
    pub async fn execute(&self, i: AddMemberInput) -> Result<(), UseCaseError> {
        let family = self
            .repo
            .get_family(i.family_id)
            .await?
            .ok_or(UseCaseError::NotFound)?;
        let now = self.clock.now();
        let member = FamilyMember {
            family_id: i.family_id,
            user_id: i.user_id,
            role: i.role,
            joined_at: now,
        };
        self.repo.add_member(&member).await?;
        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: i.actor_id,
                action: "family.member.added".into(),
                entity_type: "FamilyMember".into(),
                entity_id: format!("{}/{}", i.family_id, i.user_id),
                before_hash: hash_state(&family),
                after_hash: hash_state(&member),
            })
            .await?;
        Ok(())
    }
}

pub struct GetFamily {
    repo: Arc<dyn FamilyRepository>,
}
impl GetFamily {
    pub fn new(repo: Arc<dyn FamilyRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(&self, id: EntityId<Family>) -> Result<Family, UseCaseError> {
        self.repo
            .get_family(id)
            .await?
            .ok_or(UseCaseError::NotFound)
    }
}

pub struct ListMembers {
    repo: Arc<dyn FamilyRepository>,
}
impl ListMembers {
    pub fn new(repo: Arc<dyn FamilyRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(&self, id: EntityId<Family>) -> Result<Vec<FamilyMember>, UseCaseError> {
        Ok(self.repo.list_members(id).await?)
    }
}

pub struct ListFamiliesForUser {
    repo: Arc<dyn FamilyRepository>,
}
impl ListFamiliesForUser {
    pub fn new(repo: Arc<dyn FamilyRepository>) -> Self {
        Self { repo }
    }
    pub async fn execute(&self, user_id: EntityId<UserRef>) -> Result<Vec<Family>, UseCaseError> {
        Ok(self.repo.families_for_user(user_id).await?)
    }
}
