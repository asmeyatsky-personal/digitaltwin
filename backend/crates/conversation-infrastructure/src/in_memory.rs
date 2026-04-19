//! In-memory adapters for application-layer tests (§3.2).

use async_trait::async_trait;
use conversation_domain::{
    Conversation, EmotionalTone, Message,
    conversation::UserRef,
    ports::{
        ConversationRepository, LlmError, LlmPort, LlmResponse, MessageStore, RepositoryError,
    },
};
use kernel::EntityId;
use std::{collections::HashMap, sync::Mutex};

#[derive(Default)]
pub struct InMemoryConversationRepository {
    inner: Mutex<HashMap<EntityId<Conversation>, Conversation>>,
}

#[async_trait]
impl ConversationRepository for InMemoryConversationRepository {
    async fn find_by_id(
        &self,
        id: EntityId<Conversation>,
    ) -> Result<Option<Conversation>, RepositoryError> {
        Ok(self
            .inner
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?
            .get(&id)
            .cloned())
    }

    async fn list_for_user(
        &self,
        user_id: EntityId<UserRef>,
        limit: u32,
    ) -> Result<Vec<Conversation>, RepositoryError> {
        let guard = self
            .inner
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        Ok(guard
            .values()
            .filter(|c| c.user_id() == user_id)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn insert(&self, c: &Conversation) -> Result<(), RepositoryError> {
        self.inner
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?
            .insert(c.id(), c.clone());
        Ok(())
    }

    async fn update(&self, c: &Conversation) -> Result<(), RepositoryError> {
        self.inner
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?
            .insert(c.id(), c.clone());
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryMessageStore {
    inner: Mutex<HashMap<EntityId<Conversation>, Vec<Message>>>,
}

#[async_trait]
impl MessageStore for InMemoryMessageStore {
    async fn append(
        &self,
        conversation_id: EntityId<Conversation>,
        message: &Message,
    ) -> Result<(), RepositoryError> {
        self.inner
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?
            .entry(conversation_id)
            .or_default()
            .push(message.clone());
        Ok(())
    }

    async fn history(
        &self,
        conversation_id: EntityId<Conversation>,
        limit: u32,
    ) -> Result<Vec<Message>, RepositoryError> {
        let guard = self
            .inner
            .lock()
            .map_err(|e| RepositoryError::Backend(e.to_string()))?;
        Ok(guard
            .get(&conversation_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .take(limit as usize)
            .collect())
    }
}

/// Deterministic LLM for tests. Echoes the input with a fixed tone.
pub struct EchoLlm {
    pub tone: EmotionalTone,
}

#[async_trait]
impl LlmPort for EchoLlm {
    async fn reply(
        &self,
        _history: &[Message],
        user_message: &str,
    ) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            body: format!("echo: {user_message}"),
            tone: self.tone,
            model: "test-echo".into(),
            tokens_in: user_message.len() as u32,
            tokens_out: user_message.len() as u32,
        })
    }
}
