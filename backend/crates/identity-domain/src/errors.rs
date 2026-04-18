use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("email is invalid")]
    InvalidEmail,
    #[error("password does not meet strength requirements")]
    WeakPassword,
    #[error("user is not active")]
    UserInactive,
    #[error("credentials are invalid")]
    InvalidCredentials,
}
