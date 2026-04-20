use thiserror::Error;

#[derive(Debug, Error)]
pub enum FirestoreError {
    #[error("auth failed: {0}")]
    Auth(String),
    #[error("http error: {0}")]
    Http(String),
    #[error("http status {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("decode: {0}")]
    Decode(String),
    #[error("not found")]
    NotFound,
    #[error("call timed out")]
    Timeout,
}
