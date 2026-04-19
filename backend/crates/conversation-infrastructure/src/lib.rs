//! Layer: infrastructure (Conversation bounded context).
//! Ports implemented: `ConversationRepository`, `MessageStore`, `LlmPort`.
//! MCP integration: none here; MCP server lives in `conversation-presentation`.
//! Stack choice: canonical. Postgres for conversation metadata and message
//! transcript (ADR-0003: Firestore is the canonical home for message
//! documents; the Postgres `messages` table is a temporary shim until the
//! Firestore adapter lands — same port, different adapter).

#![forbid(unsafe_code)]
#![deny(clippy::all)]

pub mod anthropic_llm;
pub mod in_memory;
pub mod postgres_conversation_repository;
pub mod postgres_message_store;

pub use anthropic_llm::AnthropicLlm;
pub use postgres_conversation_repository::PostgresConversationRepository;
pub use postgres_message_store::PostgresMessageStore;
