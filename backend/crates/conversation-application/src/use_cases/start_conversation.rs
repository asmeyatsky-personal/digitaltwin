use audit::{Actor, AuditEvent, AuditPort, hash_state};
use conversation_domain::{
    Conversation,
    conversation::UserRef,
    ports::{ConversationRepository, RepositoryError},
};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

pub struct StartConversationInput {
    pub user_id: EntityId<UserRef>,
    pub title: Option<String>,
    pub actor_id: EntityId<Actor>,
}

pub struct StartConversationOutput {
    pub conversation_id: EntityId<Conversation>,
}

#[derive(Debug, Error)]
pub enum StartConversationError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
}

pub struct StartConversation {
    repo: Arc<dyn ConversationRepository>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}

impl StartConversation {
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
    #[instrument(skip(self, input))]
    pub async fn execute(
        &self,
        input: StartConversationInput,
    ) -> Result<StartConversationOutput, StartConversationError> {
        let now = self.clock.now();
        let id = EntityId::new();
        let c = Conversation::start(id, input.user_id, input.title, now);

        self.repo.insert(&c).await?;

        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: input.actor_id,
                action: "conversation.started".into(),
                entity_type: "Conversation".into(),
                entity_id: id.to_string(),
                before_hash: String::new(),
                after_hash: hash_state(&c),
            })
            .await?;

        Ok(StartConversationOutput {
            conversation_id: id,
        })
    }
}
