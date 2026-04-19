//! Layer: presentation (Conversation bounded context). gRPC + REST + MCP.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

pub mod grpc;
pub mod mcp;
pub mod rest;

use std::sync::Arc;

use conversation_application::{
    EndConversation, GetHistory, ListConversations, SendMessage, StartConversation,
};

#[derive(Clone)]
pub struct ConversationServices {
    pub start: Arc<StartConversation>,
    pub send: Arc<SendMessage>,
    pub end: Arc<EndConversation>,
    pub history: Arc<GetHistory>,
    pub list: Arc<ListConversations>,
}
