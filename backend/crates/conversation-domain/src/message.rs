use crate::{emotion::EmotionalTone, errors::DomainError};
use chrono::{DateTime, Utc};
use kernel::EntityId;
use serde::Serialize;

pub struct MessageIdMarker;
pub type MessageId = EntityId<MessageIdMarker>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
}

const MAX_MESSAGE_CHARS: usize = 8_000;

/// Immutable message value object. Invariants checked in `new`.
#[derive(Debug, Clone, Serialize)]
pub struct Message {
    id: MessageId,
    role: Role,
    body: String,
    tone: EmotionalTone,
    sent_at: DateTime<Utc>,
}

impl Message {
    /// # Errors
    /// `EmptyMessage` / `MessageTooLong` on body validation failure.
    pub fn new(
        id: MessageId,
        role: Role,
        body: String,
        tone: EmotionalTone,
        sent_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if body.trim().is_empty() {
            return Err(DomainError::EmptyMessage);
        }
        if body.chars().count() > MAX_MESSAGE_CHARS {
            return Err(DomainError::MessageTooLong {
                max: MAX_MESSAGE_CHARS,
            });
        }
        Ok(Self {
            id,
            role,
            body,
            tone,
            sent_at,
        })
    }

    #[must_use]
    pub fn id(&self) -> MessageId {
        self.id
    }
    #[must_use]
    pub fn role(&self) -> Role {
        self.role
    }
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
    #[must_use]
    pub fn tone(&self) -> EmotionalTone {
        self.tone
    }
    #[must_use]
    pub fn sent_at(&self) -> DateTime<Utc> {
        self.sent_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_body() {
        let e = Message::new(
            MessageId::new(),
            Role::User,
            "   ".into(),
            EmotionalTone::Neutral,
            Utc::now(),
        );
        assert_eq!(e.err(), Some(DomainError::EmptyMessage));
    }

    #[test]
    fn rejects_overlong_body() {
        let body = "a".repeat(MAX_MESSAGE_CHARS + 1);
        let e = Message::new(
            MessageId::new(),
            Role::User,
            body,
            EmotionalTone::Neutral,
            Utc::now(),
        );
        assert!(matches!(e, Err(DomainError::MessageTooLong { .. })));
    }
}
