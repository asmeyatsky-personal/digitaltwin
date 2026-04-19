//! Postgres adapter for `MessageStore`. Transcripts are document-shaped data
//! that ADR-0003 earmarks for Firestore; we ship this Postgres shim so the
//! service is functional end-to-end, and swap to a `FirestoreMessageStore`
//! once the Firestore client wiring lands. The domain port stays the same —
//! adapters are an implementation detail.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use conversation_domain::{
    Conversation, EmotionalTone, Message, MessageId, Role,
    ports::{MessageStore, RepositoryError},
};
use kernel::EntityId;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresMessageStore {
    pool: PgPool,
}

impl PostgresMessageStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn role_str(r: Role) -> &'static str {
        match r {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        }
    }

    fn map_row(row: PgRow) -> Result<Message, RepositoryError> {
        let id: Uuid = row
            .try_get("id")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let role: String = row
            .try_get("role")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let body: String = row
            .try_get("body")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let tone: String = row
            .try_get("tone")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        let sent_at: DateTime<Utc> = row
            .try_get("sent_at")
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;

        let role = match role.as_str() {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "system" => Role::System,
            other => return Err(RepositoryError::Backend(format!("unknown role: {other}"))),
        };
        let tone = EmotionalTone::parse(&tone)
            .map_err(|e| RepositoryError::Backend(format!("bad tone: {e}")))?;

        Message::new(MessageId::from_uuid(id), role, body, tone, sent_at)
            .map_err(|e| RepositoryError::Backend(format!("rehydrate: {e}")))
    }
}

#[async_trait]
impl MessageStore for PostgresMessageStore {
    async fn append(
        &self,
        conversation_id: EntityId<Conversation>,
        message: &Message,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO conversation.messages \
             (id, conversation_id, role, body, tone, sent_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(message.id().as_uuid())
        .bind(conversation_id.as_uuid())
        .bind(Self::role_str(message.role()))
        .bind(message.body())
        .bind(message.tone().as_str())
        .bind(message.sent_at())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn history(
        &self,
        conversation_id: EntityId<Conversation>,
        limit: u32,
    ) -> Result<Vec<Message>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, role, body, tone, sent_at \
             FROM conversation.messages WHERE conversation_id = $1 \
             ORDER BY sent_at ASC LIMIT $2",
        )
        .bind(conversation_id.as_uuid())
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        rows.into_iter().map(Self::map_row).collect()
    }
}
