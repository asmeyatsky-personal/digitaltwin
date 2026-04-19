use crate::errors::DomainError;
use chrono::{DateTime, Utc};
use kernel::EntityId;
use serde::Serialize;

/// Zero-sized marker for User ids, owned by the identity context. Conversation
/// holds them as opaque `EntityId<UserRef>` so we never confuse them with
/// Conversation ids.
pub struct UserRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationStatus {
    Active,
    Ended,
}

/// Conversation aggregate root. Metadata only — the message transcript lives
/// in Firestore per ADR-0003 (documents store).
#[derive(Debug, Clone, Serialize)]
pub struct Conversation {
    id: EntityId<Conversation>,
    user_id: EntityId<UserRef>,
    title: Option<String>,
    status: ConversationStatus,
    message_count: u32,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
}

impl Conversation {
    #[must_use]
    pub fn start(
        id: EntityId<Conversation>,
        user_id: EntityId<UserRef>,
        title: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            title,
            status: ConversationStatus::Active,
            message_count: 0,
            started_at: now,
            ended_at: None,
        }
    }

    #[must_use]
    pub fn id(&self) -> EntityId<Conversation> {
        self.id
    }
    #[must_use]
    pub fn user_id(&self) -> EntityId<UserRef> {
        self.user_id
    }
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    #[must_use]
    pub fn status(&self) -> ConversationStatus {
        self.status
    }
    #[must_use]
    pub fn message_count(&self) -> u32 {
        self.message_count
    }
    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }
    #[must_use]
    pub fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.ended_at
    }

    /// # Errors
    /// `ConversationEnded` if the aggregate is no longer active.
    pub fn record_message_pair(&self) -> Result<Self, DomainError> {
        if self.status == ConversationStatus::Ended {
            return Err(DomainError::ConversationEnded);
        }
        Ok(Self {
            message_count: self.message_count.saturating_add(2),
            ..self.clone()
        })
    }

    #[must_use]
    pub fn end(&self, now: DateTime<Utc>) -> Self {
        Self {
            status: ConversationStatus::Ended,
            ended_at: Some(now),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Conversation {
        Conversation::start(
            EntityId::new(),
            EntityId::new(),
            Some("test".into()),
            Utc::now(),
        )
    }

    #[test]
    fn end_is_immutable() {
        let c = fixture();
        let ended = c.end(Utc::now());
        assert_eq!(c.status(), ConversationStatus::Active);
        assert_eq!(ended.status(), ConversationStatus::Ended);
    }

    #[test]
    fn ended_conversation_rejects_messages() {
        let c = fixture().end(Utc::now());
        assert_eq!(
            c.record_message_pair().err(),
            Some(DomainError::ConversationEnded)
        );
    }

    #[test]
    fn record_message_pair_increments_by_two() {
        let c = fixture();
        let c = c.record_message_pair().expect("ok");
        assert_eq!(c.message_count(), 2);
        let c = c.record_message_pair().expect("ok");
        assert_eq!(c.message_count(), 4);
    }

    #[test]
    fn getters_expose_start_state() {
        let c = fixture();
        assert_eq!(c.title(), Some("test"));
        assert_eq!(c.message_count(), 0);
        assert!(c.ended_at().is_none());
        let _ = c.id();
        let _ = c.user_id();
        assert!(c.started_at() <= Utc::now());
    }

    #[test]
    fn end_sets_ended_at() {
        let c = fixture();
        let now = Utc::now();
        let ended = c.end(now);
        assert_eq!(ended.ended_at(), Some(now));
    }
}
