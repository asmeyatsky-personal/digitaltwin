//! Ports implemented in `conversation-infrastructure` and consumed by
//! `conversation-application`. Tests use in-memory adapters (§3.2).

use crate::{
    conversation::{Conversation, UserRef},
    emotion::EmotionalTone,
    errors::DomainError,
    message::Message,
};
use kernel::EntityId;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("backend error: {0}")]
    Backend(String),
    #[error("not found")]
    NotFound,
}

#[async_trait::async_trait]
pub trait ConversationRepository: Send + Sync {
    async fn find_by_id(
        &self,
        id: EntityId<Conversation>,
    ) -> Result<Option<Conversation>, RepositoryError>;
    async fn list_for_user(
        &self,
        user_id: EntityId<UserRef>,
        limit: u32,
    ) -> Result<Vec<Conversation>, RepositoryError>;
    async fn insert(&self, c: &Conversation) -> Result<(), RepositoryError>;
    async fn update(&self, c: &Conversation) -> Result<(), RepositoryError>;
}

#[async_trait::async_trait]
pub trait MessageStore: Send + Sync {
    async fn append(
        &self,
        conversation_id: EntityId<Conversation>,
        message: &Message,
    ) -> Result<(), RepositoryError>;
    async fn history(
        &self,
        conversation_id: EntityId<Conversation>,
        limit: u32,
    ) -> Result<Vec<Message>, RepositoryError>;
}

/// LLM adapter port. Responses are validated against `EmotionalTone` — §4
/// "AI output that mutates state must be validated against an explicit schema".
pub struct LlmResponse {
    pub body: String,
    pub tone: EmotionalTone,
    pub model: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("llm call failed: {0}")]
    CallFailed(String),
    #[error("llm response did not match schema: {0}")]
    SchemaViolation(#[from] DomainError),
    #[error("llm call timed out")]
    Timeout,
}

#[async_trait::async_trait]
pub trait LlmPort: Send + Sync {
    async fn reply(&self, history: &[Message], user_message: &str)
    -> Result<LlmResponse, LlmError>;
}
