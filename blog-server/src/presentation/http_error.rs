//! Переводит доменную ошибку в HTTP-ответ с телом `{ "error": "..." }` и подходящим статусом.

use actix_web::{http::StatusCode, HttpResponse};
use serde_json::json;

use crate::domain::error::DomainError;

/// Выбирает код ответа и сериализует человекочитаемое сообщение из варианта `DomainError`.
pub fn domain_error_response(err: DomainError) -> HttpResponse {
    let status = match &err {
        DomainError::Validation(_) => StatusCode::BAD_REQUEST,
        DomainError::InvalidCredentials | DomainError::Unauthorized => StatusCode::UNAUTHORIZED,
        DomainError::Forbidden => StatusCode::FORBIDDEN,
        DomainError::UserNotFound => StatusCode::NOT_FOUND,
        DomainError::PostNotFound => StatusCode::NOT_FOUND,
        DomainError::UserAlreadyExists => StatusCode::CONFLICT,
        DomainError::DataBaseInternal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };

    HttpResponse::build(status).json(json!({ "error": err.to_string() }))
}
