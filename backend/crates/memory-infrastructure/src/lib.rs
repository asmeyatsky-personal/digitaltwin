//! Layer: infrastructure (Memory bounded context). Firestore per ADR-0003.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use firestore_client::{Document, FirestoreClient};
use kernel::EntityId;
use memory_domain::{
    LifeEvent, LifeEventCategory, LifeEventStore, Memory, MemoryStore, StoreError, UserRef,
};
use serde_json::{Value, json};
use std::{str::FromStr, sync::Mutex};

pub struct FirestoreMemoryStore {
    client: FirestoreClient,
}
impl FirestoreMemoryStore {
    #[must_use]
    pub fn new(client: FirestoreClient) -> Self { Self { client } }
}

#[async_trait]
impl MemoryStore for FirestoreMemoryStore {
    async fn save(&self, m: &Memory) -> Result<(), StoreError> {
        let fields = json!({
            "user_id": m.user_id.to_string(),
            "content": m.content,
            "mood": m.mood,
            "tags": m.tags.clone(),
            "created_at": m.created_at.to_rfc3339(),
        });
        self.client
            .set("memories", &m.id.to_string(), fields)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))
    }

    async fn list_for_user(&self, user_id: EntityId<UserRef>, limit: u32) -> Result<Vec<Memory>, StoreError> {
        let docs = self
            .client
            .list("memories", limit)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(docs
            .into_iter()
            .filter_map(|d| doc_to_memory(&d, user_id).ok())
            .collect())
    }
}

fn doc_to_memory(d: &Document, user_id: EntityId<UserRef>) -> Result<Memory, StoreError> {
    let id = EntityId::from_str(&d.id).map_err(|e| StoreError::Backend(e.to_string()))?;
    let uid_s = d.fields.get("user_id").and_then(Value::as_str).unwrap_or_default();
    let parsed_user = EntityId::<UserRef>::from_str(uid_s).unwrap_or(user_id);
    let content = d.fields.get("content").and_then(Value::as_str).unwrap_or("").to_string();
    let mood = d.fields.get("mood").and_then(Value::as_str).unwrap_or("").to_string();
    let tags: Vec<String> = d
        .fields
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|t| t.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    let created_at = d
        .fields
        .get("created_at")
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    Memory::new(id, parsed_user, content, mood, tags, created_at)
        .map_err(|e| StoreError::Backend(e.to_string()))
}

pub struct FirestoreLifeEventStore {
    client: FirestoreClient,
}
impl FirestoreLifeEventStore {
    #[must_use]
    pub fn new(client: FirestoreClient) -> Self { Self { client } }
}

#[async_trait]
impl LifeEventStore for FirestoreLifeEventStore {
    async fn save(&self, e: &LifeEvent) -> Result<(), StoreError> {
        let fields = json!({
            "user_id": e.user_id.to_string(),
            "title": e.title,
            "description": e.description,
            "event_date": e.event_date.to_rfc3339(),
            "category": e.category.as_str(),
            "emotional_impact": e.emotional_impact as i64,
            "is_recurring": e.is_recurring,
        });
        self.client
            .set("life_events", &e.id.to_string(), fields)
            .await
            .map_err(|err| StoreError::Backend(err.to_string()))
    }

    async fn timeline(&self, user_id: EntityId<UserRef>, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<LifeEvent>, StoreError> {
        let docs = self
            .client
            .list("life_events", 200)
            .await
            .map_err(|err| StoreError::Backend(err.to_string()))?;
        Ok(docs
            .into_iter()
            .filter_map(|d| doc_to_event(&d, user_id).ok())
            .filter(|e| e.event_date >= from && e.event_date <= to)
            .collect())
    }

    async fn upcoming(&self, user_id: EntityId<UserRef>, horizon_days: u32) -> Result<Vec<LifeEvent>, StoreError> {
        let now = Utc::now();
        let horizon = now + ChronoDuration::days(i64::from(horizon_days));
        self.timeline(user_id, now, horizon).await
    }
}

fn doc_to_event(d: &Document, user_id: EntityId<UserRef>) -> Result<LifeEvent, StoreError> {
    let id = EntityId::from_str(&d.id).map_err(|e| StoreError::Backend(e.to_string()))?;
    let uid_s = d.fields.get("user_id").and_then(Value::as_str).unwrap_or_default();
    let parsed_user = EntityId::<UserRef>::from_str(uid_s).unwrap_or(user_id);
    let title = d.fields.get("title").and_then(Value::as_str).unwrap_or("").to_string();
    let description = d.fields.get("description").and_then(Value::as_str).unwrap_or("").to_string();
    let event_date = d
        .fields
        .get("event_date")
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let category = d
        .fields
        .get("category")
        .and_then(Value::as_str)
        .and_then(|c| LifeEventCategory::parse(c).ok())
        .unwrap_or(LifeEventCategory::Other);
    let emotional_impact = d.fields.get("emotional_impact").and_then(Value::as_i64).unwrap_or(0) as i32;
    let is_recurring = d.fields.get("is_recurring").and_then(Value::as_bool).unwrap_or(false);
    LifeEvent::new(id, parsed_user, title, description, event_date, category, emotional_impact, is_recurring)
        .map_err(|e| StoreError::Backend(e.to_string()))
}

// ---- In-memory adapters for tests ----

#[derive(Default)]
pub struct InMemoryMemoryStore {
    inner: Mutex<Vec<Memory>>,
}
#[async_trait]
impl MemoryStore for InMemoryMemoryStore {
    async fn save(&self, m: &Memory) -> Result<(), StoreError> {
        self.inner.lock().map_err(|e| StoreError::Backend(e.to_string()))?.push(m.clone());
        Ok(())
    }
    async fn list_for_user(&self, user_id: EntityId<UserRef>, limit: u32) -> Result<Vec<Memory>, StoreError> {
        let g = self.inner.lock().map_err(|e| StoreError::Backend(e.to_string()))?;
        let mut matches: Vec<Memory> = g.iter().filter(|m| m.user_id == user_id).cloned().collect();
        matches.sort_by_key(|m| std::cmp::Reverse(m.created_at));
        matches.truncate(limit as usize);
        Ok(matches)
    }
}

#[derive(Default)]
pub struct InMemoryLifeEventStore {
    inner: Mutex<Vec<LifeEvent>>,
}
#[async_trait]
impl LifeEventStore for InMemoryLifeEventStore {
    async fn save(&self, e: &LifeEvent) -> Result<(), StoreError> {
        self.inner.lock().map_err(|er| StoreError::Backend(er.to_string()))?.push(e.clone());
        Ok(())
    }
    async fn timeline(&self, user_id: EntityId<UserRef>, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<LifeEvent>, StoreError> {
        Ok(self.inner.lock().map_err(|e| StoreError::Backend(e.to_string()))?
            .iter()
            .filter(|e| e.user_id == user_id && e.event_date >= from && e.event_date <= to)
            .cloned()
            .collect())
    }
    async fn upcoming(&self, user_id: EntityId<UserRef>, horizon_days: u32) -> Result<Vec<LifeEvent>, StoreError> {
        let now = Utc::now();
        self.timeline(user_id, now, now + ChronoDuration::days(i64::from(horizon_days))).await
    }
}
