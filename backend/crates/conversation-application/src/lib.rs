//! Layer: application (Conversation bounded context).
//! Ports: consumes `conversation_domain::ports::{ConversationRepository,
//! MessageStore, LlmPort}` and `audit::AuditPort`.
//! MCP integration: Conversation MCP tools/resources call these use cases.
//! Stack choice: canonical.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

pub mod use_cases;

pub use use_cases::{
    end_conversation::{EndConversation, EndConversationError, EndConversationInput},
    get_history::{GetHistory, GetHistoryError, GetHistoryInput, GetHistoryOutput},
    list_conversations::{
        ListConversations, ListConversationsError, ListConversationsInput, ListConversationsOutput,
    },
    send_message::{SendMessage, SendMessageError, SendMessageInput, SendMessageOutput},
    start_conversation::{
        StartConversation, StartConversationError, StartConversationInput, StartConversationOutput,
    },
};
