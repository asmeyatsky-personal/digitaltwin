use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("invalid emotional tone: {0}")]
    InvalidEmotion(String),
    #[error("message body cannot be empty")]
    EmptyMessage,
    #[error("message exceeds max length ({max} chars)")]
    MessageTooLong { max: usize },
    #[error("conversation is already ended")]
    ConversationEnded,
}
