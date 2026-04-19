use audit::{Actor, AuditEvent, AuditPort, hash_state};
use conversation_domain::{
    Conversation, DomainError, EmotionalTone, Message, MessageId, Role,
    ports::{ConversationRepository, LlmError, LlmPort, MessageStore, RepositoryError},
};
use kernel::{Clock, EntityId};
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

pub struct SendMessageInput {
    pub conversation_id: EntityId<Conversation>,
    pub body: String,
    pub actor_id: EntityId<Actor>,
}

pub struct SendMessageOutput {
    pub reply: String,
    pub tone: EmotionalTone,
}

#[derive(Debug, Error)]
pub enum SendMessageError {
    #[error("conversation not found")]
    NotFound,
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Llm(#[from] LlmError),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
}

pub struct SendMessage {
    repo: Arc<dyn ConversationRepository>,
    store: Arc<dyn MessageStore>,
    llm: Arc<dyn LlmPort>,
    audit: Arc<dyn AuditPort>,
    clock: Arc<dyn Clock>,
}

impl SendMessage {
    #[must_use]
    pub fn new(
        repo: Arc<dyn ConversationRepository>,
        store: Arc<dyn MessageStore>,
        llm: Arc<dyn LlmPort>,
        audit: Arc<dyn AuditPort>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            store,
            llm,
            audit,
            clock,
        }
    }

    /// # Errors
    /// See variants.
    #[instrument(skip(self, input), fields(conversation_id = %input.conversation_id))]
    pub async fn execute(
        &self,
        input: SendMessageInput,
    ) -> Result<SendMessageOutput, SendMessageError> {
        let now = self.clock.now();
        let convo = self
            .repo
            .find_by_id(input.conversation_id)
            .await?
            .ok_or(SendMessageError::NotFound)?;

        // Reuse the domain invariant: ended conversations reject new messages.
        let updated = convo.record_message_pair()?;

        // Store user message first so the LLM call sees the full transcript.
        let user_msg = Message::new(
            MessageId::new(),
            Role::User,
            input.body.clone(),
            EmotionalTone::Neutral, // facial/voice signals fuse in separately.
            now,
        )?;
        self.store.append(updated.id(), &user_msg).await?;

        // Pull the history the LLM should see. Bounded to 50 to cap prompt size
        // and cost; longer context moves to the Memory bounded context later.
        let history = self.store.history(updated.id(), 50).await?;

        // LLM call. Adapter is responsible for timeout + circuit breaker (§4).
        let llm_out = self.llm.reply(&history, &input.body).await?;

        // §4: AI output that mutates state must be validated against an explicit
        // schema. The `LlmAdapter` already parses via EmotionalTone::parse so
        // `llm_out.tone` is known-good by the time we reach here.
        let assistant_msg = Message::new(
            MessageId::new(),
            Role::Assistant,
            llm_out.body.clone(),
            llm_out.tone,
            self.clock.now(),
        )?;
        self.store.append(updated.id(), &assistant_msg).await?;

        self.repo.update(&updated).await?;

        self.audit
            .append(AuditEvent {
                occurred_at: now,
                actor_id: input.actor_id,
                action: "conversation.message.sent".into(),
                entity_type: "Conversation".into(),
                entity_id: updated.id().to_string(),
                before_hash: hash_state(&convo),
                after_hash: hash_state(&updated),
            })
            .await?;

        // Per-AI-call attributes (§6).
        tracing::info!(
            model = %llm_out.model,
            tokens_in = llm_out.tokens_in,
            tokens_out = llm_out.tokens_out,
            tone = %llm_out.tone.as_str(),
            "llm reply"
        );

        Ok(SendMessageOutput {
            reply: llm_out.body,
            tone: llm_out.tone,
        })
    }
}
