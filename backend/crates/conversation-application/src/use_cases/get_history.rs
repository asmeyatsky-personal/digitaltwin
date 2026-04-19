use conversation_domain::{
    Conversation, Message,
    ports::{MessageStore, RepositoryError},
};
use kernel::EntityId;
use std::sync::Arc;
use thiserror::Error;

pub struct GetHistoryInput {
    pub conversation_id: EntityId<Conversation>,
    pub limit: u32,
}

pub struct GetHistoryOutput {
    pub messages: Vec<Message>,
}

#[derive(Debug, Error)]
pub enum GetHistoryError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

pub struct GetHistory {
    store: Arc<dyn MessageStore>,
}

impl GetHistory {
    #[must_use]
    pub fn new(store: Arc<dyn MessageStore>) -> Self {
        Self { store }
    }

    /// # Errors
    /// Propagates `RepositoryError` from the message store.
    pub async fn execute(
        &self,
        input: GetHistoryInput,
    ) -> Result<GetHistoryOutput, GetHistoryError> {
        let messages = self
            .store
            .history(input.conversation_id, input.limit)
            .await?;
        Ok(GetHistoryOutput { messages })
    }
}
