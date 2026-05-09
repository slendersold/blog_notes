//! Перечень ошибок доменной логики для единого сопоставления с HTTP-кодами и gRPC статусами.

use thiserror::Error;

/// Бизнес-ошибки, которые сервисы возвращают наружу вместо сырых технических ошибок БД или сети.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("{0}")]
    Validation(String),
    #[error("user not found")]
    UserNotFound,
    #[error("user already exists")]
    UserAlreadyExists,
    #[error("invalid credentials")]
    InvalidCredentials,
    /// Нет секрета, битый токен или неверная подпись.
    #[error("unauthorized")]
    Unauthorized,
    #[error("post not found")]
    PostNotFound,
    #[error("forbidden")]
    Forbidden,
    /// Внутренняя техническая ошибка при работе с БД (до детализированной обработки).
    #[error("database error: {0}")]
    DataBaseInternal(String),
}
