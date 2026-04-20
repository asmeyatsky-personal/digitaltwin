use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("confidence must be in [0.0, 1.0], got {0}")]
    InvalidConfidence(String),
    #[error("unknown modality: {0}")]
    UnknownModality(String),
    #[error("unknown tone: {0}")]
    UnknownTone(String),
    #[error("no readings to fuse")]
    EmptyReadings,
}
