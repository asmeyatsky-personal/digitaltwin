use conversation_domain::{
    Conversation,
    conversation::UserRef,
    ports::{ConversationRepository, RepositoryError},
};
use kernel::EntityId;
use std::sync::Arc;
use thiserror::Error;

pub struct ListConversationsInput {
    pub user_id: EntityId<UserRef>,
    pub limit: u32,
}

pub struct ListConversationsOutput {
    pub conversations: Vec<Conversation>,
}

#[derive(Debug, Error)]
pub enum ListConversationsError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

pub struct ListConversations {
    repo: Arc<dyn ConversationRepository>,
}

impl ListConversations {
    #[must_use]
    pub fn new(repo: Arc<dyn ConversationRepository>) -> Self {
        Self { repo }
    }

    /// # Errors
    /// Propagates `RepositoryError` from the conversation repository.
    pub async fn execute(
        &self,
        input: ListConversationsInput,
    ) -> Result<ListConversationsOutput, ListConversationsError> {
        let conversations = self.repo.list_for_user(input.user_id, input.limit).await?;
        Ok(ListConversationsOutput { conversations })
    }
}
