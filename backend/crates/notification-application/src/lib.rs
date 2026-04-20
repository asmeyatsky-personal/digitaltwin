//! Application (Notification bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::{Actor, AuditEvent, AuditPort, hash_state};
use kernel::{Clock, EntityId};
use notification_domain::{DeviceToken, DomainError, Payload, Platform, PushError, PushPort, StoreError, TokenRepository, UserRef};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error(transparent)] Domain(#[from] DomainError),
    #[error(transparent)] Store(#[from] StoreError),
    #[error(transparent)] Push(#[from] PushError),
    #[error(transparent)] Audit(#[from] audit::AuditError),
}

pub struct RegisterDeviceInput {
    pub user_id: EntityId<UserRef>,
    pub token: String,
    pub platform: Platform,
    pub actor_id: EntityId<Actor>,
}
pub struct RegisterDevice { repo: Arc<dyn TokenRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock> }
impl RegisterDevice {
    pub fn new(repo: Arc<dyn TokenRepository>, audit: Arc<dyn AuditPort>, clock: Arc<dyn Clock>) -> Self { Self { repo, audit, clock } }
    pub async fn execute(&self, i: RegisterDeviceInput) -> Result<(), UseCaseError> {
        let now = self.clock.now();
        let t = DeviceToken::new(i.user_id, i.token, i.platform, now)?;
        self.repo.register(&t).await?;
        self.audit.append(AuditEvent {
            occurred_at: now, actor_id: i.actor_id,
            action: "notification.device.registered".into(),
            entity_type: "DeviceToken".into(),
            entity_id: format!("{}/{}", i.user_id, t.platform.as_str()),
            before_hash: String::new(), after_hash: hash_state(&t),
        }).await?;
        Ok(())
    }
}

pub struct UnregisterDevice { repo: Arc<dyn TokenRepository> }
impl UnregisterDevice {
    pub fn new(repo: Arc<dyn TokenRepository>) -> Self { Self { repo } }
    pub async fn execute(&self, token: &str) -> Result<(), UseCaseError> {
        self.repo.unregister(token).await?;
        Ok(())
    }
}

pub struct SendPushInput {
    pub user_id: EntityId<UserRef>,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
}
pub struct SendPush { repo: Arc<dyn TokenRepository>, push: Arc<dyn PushPort> }
impl SendPush {
    pub fn new(repo: Arc<dyn TokenRepository>, push: Arc<dyn PushPort>) -> Self { Self { repo, push } }
    pub async fn execute(&self, i: SendPushInput) -> Result<u32, UseCaseError> {
        let tokens = self.repo.for_user(i.user_id).await?;
        let n = u32::try_from(tokens.len()).unwrap_or(u32::MAX);
        if tokens.is_empty() { return Ok(0); }
        self.push.send(&tokens, &Payload { title: i.title, body: i.body, data: i.data }).await?;
        Ok(n)
    }
}
