//! Layer: domain (Conversation bounded context).
//! Ports: `ConversationRepository`, `MessageStore`, `LlmPort`.
//! MCP integration: use cases exposed via the Conversation MCP server; tools
//! are write-side use cases (SendMessage, EndConversation); resources are
//! read-side (conversation history).
//! Stack choice: canonical.
//!
//! Pure domain for the emotional companion's conversation aggregate. Uses
//! the unified `EmotionalTone` enum from AD-1 (8 values) so taxonomy is no
//! longer lossy at service boundaries.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::module_name_repetitions)]

pub mod conversation;
pub mod emotion;
pub mod errors;
pub mod message;
pub mod ports;

pub use conversation::{Conversation, ConversationStatus};
pub use emotion::EmotionalTone;
pub use errors::DomainError;
pub use message::{Message, MessageId, Role};
