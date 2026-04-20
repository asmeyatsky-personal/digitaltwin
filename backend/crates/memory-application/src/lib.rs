//! Layer: application (Memory bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::{Actor, AuditEvent, AuditPort, hash_state};
use chrono::{DateTime, Utc};
use kernel::{Clock, EntityId};
use memory_domain::{
    DomainError, LifeEvent, LifeEventCategory, LifeEventStore, Memory, MemoryStore, StoreError,
    UserRef,
};
use std::sync::Arc;
use thiserror::Error;

pub struct RecordMemoryInput {
    pub user_id: EntityId<UserRef>,
    pub content: String,
    pub mood: String,
    pub tags: Vec<String>,
    pub actor_id: EntityId<Actor>,
}

pub struct RecordMemoryOutput {
    pub memory_id: EntityId<Memory>,
}

#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
}

pub struct RecordMemory {
    store: Arc<dyn MemoryStore>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}
impl RecordMemory {
    pub fn new(
        store: Arc<dyn MemoryStore>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            audit,
            clock,
        }
    }
    pub async fn execute(&self, i: RecordMemoryInput) -> Result<RecordMemoryOutput, UseCaseError> {
        let now = self.clock.now();
        let id = EntityId::<Memory>::new();
        let m = Memory::new(id, i.user_id, i.content, i.mood, i.tags, now)?;
        self.store.save(&m).await?;
        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: i.actor_id,
                action: "memory.recorded".into(),
                entity_type: "Memory".into(),
                entity_id: id.to_string(),
                before_hash: String::new(),
                after_hash: hash_state(&m),
            })
            .await?;
        Ok(RecordMemoryOutput { memory_id: id })
    }
}

pub struct GetTimeline {
    store: Arc<dyn MemoryStore>,
}
impl GetTimeline {
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        Self { store }
    }
    pub async fn execute(
        &self,
        user_id: EntityId<UserRef>,
        limit: u32,
    ) -> Result<Vec<Memory>, UseCaseError> {
        Ok(self.store.list_for_user(user_id, limit).await?)
    }
}

pub struct AddLifeEventInput {
    pub user_id: EntityId<UserRef>,
    pub title: String,
    pub description: String,
    pub event_date: DateTime<Utc>,
    pub category: LifeEventCategory,
    pub emotional_impact: i32,
    pub is_recurring: bool,
    pub actor_id: EntityId<Actor>,
}

pub struct AddLifeEvent {
    store: Arc<dyn LifeEventStore>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}
impl AddLifeEvent {
    pub fn new(
        store: Arc<dyn LifeEventStore>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            audit,
            clock,
        }
    }
    pub async fn execute(&self, i: AddLifeEventInput) -> Result<EntityId<LifeEvent>, UseCaseError> {
        let id = EntityId::<LifeEvent>::new();
        let event = LifeEvent::new(
            id,
            i.user_id,
            i.title,
            i.description,
            i.event_date,
            i.category,
            i.emotional_impact,
            i.is_recurring,
        )?;
        self.store.save(&event).await?;
        let now = self.clock.now();
        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: i.actor_id,
                action: "memory.life_event.added".into(),
                entity_type: "LifeEvent".into(),
                entity_id: id.to_string(),
                before_hash: String::new(),
                after_hash: hash_state(&event),
            })
            .await?;
        Ok(id)
    }
}

pub struct GetUpcoming {
    store: Arc<dyn LifeEventStore>,
}
impl GetUpcoming {
    pub fn new(store: Arc<dyn LifeEventStore>) -> Self {
        Self { store }
    }
    pub async fn execute(
        &self,
        user_id: EntityId<UserRef>,
        horizon_days: u32,
    ) -> Result<Vec<LifeEvent>, UseCaseError> {
        Ok(self.store.upcoming(user_id, horizon_days).await?)
    }
}

pub struct GetConversationContext {
    memories: Arc<dyn MemoryStore>,
    events: Arc<dyn LifeEventStore>,
    clock: Arc<dyn Clock>,
}
impl GetConversationContext {
    pub fn new(
        memories: Arc<dyn MemoryStore>,
        events: Arc<dyn LifeEventStore>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            memories,
            events,
            clock,
        }
    }
    /// Return the compact context snippet the Conversation LLM adapter
    /// prepends to its system prompt. Includes the 3 most recent memories
    /// and any life events in the next 14 days (for anniversary awareness).
    pub async fn execute(&self, user_id: EntityId<UserRef>) -> Result<String, UseCaseError> {
        let memories = self.memories.list_for_user(user_id, 3).await?;
        let upcoming = self.events.upcoming(user_id, 14).await?;
        let now = self.clock.now();
        let mut out = String::new();
        out.push_str(&format!("Context at {}\n", now.to_rfc3339()));
        if !memories.is_empty() {
            out.push_str("Recent memories:\n");
            for m in &memories {
                out.push_str(&format!("- {} ({})\n", m.content, m.mood));
            }
        }
        if !upcoming.is_empty() {
            out.push_str("Upcoming events:\n");
            for e in &upcoming {
                out.push_str(&format!(
                    "- {} on {} ({})\n",
                    e.title,
                    e.event_date.date_naive(),
                    e.category.as_str()
                ));
            }
        }
        Ok(out)
    }
    // Silence unused-warning; `clock` is a design placeholder for future
    // timezone-aware projections.
    #[allow(dead_code)]
    fn _touch(&self) -> DateTime<Utc> {
        self.clock.now()
    }
}
