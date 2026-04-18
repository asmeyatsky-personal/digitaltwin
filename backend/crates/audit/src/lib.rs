//! Layer: infrastructure (cross-cutting).
//! Ports: `AuditPort` (consumed by every write use case).
//! MCP integration: none directly; write-side MCP tools publish via this port.
//! Stack choice: canonical (§1: ledgers → Rust).
//!
//! Append-only audit ledger. Every domain write emits an event with actor,
//! action, and before/after content hashes. Storage is a dedicated Postgres
//! role with INSERT-only grants (§4).

#![forbid(unsafe_code)]
#![deny(clippy::all)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kernel::EntityId;
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub struct Actor;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("ledger append failed: {0}")]
    AppendFailed(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub occurred_at: DateTime<Utc>,
    pub actor_id: EntityId<Actor>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub before_hash: String,
    pub after_hash: String,
}

#[async_trait]
pub trait AuditPort: Send + Sync {
    async fn append(&self, event: AuditEvent) -> Result<(), AuditError>;
}

#[must_use]
pub fn hash_state<T: Serialize>(state: &T) -> String {
    let bytes = serde_json::to_vec(state).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}
