use audit::{Actor, AuditEvent, AuditPort, hash_state};
use conversation_domain::{
    Conversation,
    ports::{ConversationRepository, RepositoryError},
};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

pub struct EndConversationInput {
    pub conversation_id: EntityId<Conversation>,
    pub actor_id: EntityId<Actor>,
}

#[derive(Debug, Error)]
pub enum EndConversationError {
    #[error("conversation not found")]
    NotFound,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
}

pub struct EndConversation {
    repo: Arc<dyn ConversationRepository>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}

impl EndConversation {
    #[must_use]
    pub fn new(
        repo: Arc<dyn ConversationRepository>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, audit, clock }
    }

    /// # Errors
    /// See variants.
    #[instrument(skip(self, input), fields(conversation_id = %input.conversation_id))]
    pub async fn execute(&self, input: EndConversationInput) -> Result<(), EndConversationError> {
        let now = self.clock.now();
        let convo = self
            .repo
            .find_by_id(input.conversation_id)
            .await?
            .ok_or(EndConversationError::NotFound)?;

        let ended = convo.end(now);
        self.repo.update(&ended).await?;

        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: input.actor_id,
                action: "conversation.ended".into(),
                entity_type: "Conversation".into(),
                entity_id: ended.id().to_string(),
                before_hash: hash_state(&convo),
                after_hash: hash_state(&ended),
            })
            .await?;

        Ok(())
    }
}
