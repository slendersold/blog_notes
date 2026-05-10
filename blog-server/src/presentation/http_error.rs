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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::body::to_bytes;
    use actix_web::http::StatusCode;
    use crate::domain::error::DomainError;

    #[tokio::test]
    async fn validation_maps_to_400() {
        let resp = domain_error_response(DomainError::Validation("x".into()));
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn invalid_credentials_maps_to_401() {
        let resp = domain_error_response(DomainError::InvalidCredentials);
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn conflict_body_has_error_field() {
        let resp = domain_error_response(DomainError::UserAlreadyExists);
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body = to_bytes(resp.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(v["error"].is_string());
    }
}
