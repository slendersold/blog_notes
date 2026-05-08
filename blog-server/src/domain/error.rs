use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("user not found")]
    UserNotFound,
    #[error("user already exists")]
    UserAlreadyExists,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("post not found")]
    PostNotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("database error: {0}")]
    DataBaseInternal(String),
}
