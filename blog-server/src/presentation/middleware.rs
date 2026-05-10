//! JWT middleware для маршрутов под `/api/posts`: публичные `GET` / `HEAD` / `OPTIONS` без токена,
//! `POST` / `PUT` / `DELETE` — только с валидным `Authorization: Bearer`.
//!
//! Регистрация: `web::scope("/api/posts").wrap(actix_web::middleware::from_fn(jwt_posts_middleware))`.
//! Для мутаций успешная проверка кладёт `infrastructure::jwt::Claims` в расширения запроса; в хендлере используйте `web::ReqData<Claims>`.
//!
//! Сигнатура только `(ServiceRequest, Next<_>)`: иначе `Scope::wrap` в actix-web 4.13 не выводит `Transform` для `from_fn` с `Data<>` в аргументах.

use actix_web::body::BoxBody;
use actix_web::error::ErrorUnauthorized;
use actix_web::http::header::AUTHORIZATION;
use actix_web::http::Method;
use actix_web::middleware::Next;
use actix_web::web::Data;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse};
use actix_web::{Error, HttpMessage};

use crate::application::auth_service::AuthService;

/// Разбор значения заголовка `Authorization` (строка после `to_str`, уже одна линия).
pub(crate) fn bearer_from_authorization_value(value: Option<&str>) -> Option<&str> {
    let s = value?.trim();
    let t = s
        .strip_prefix("Bearer ")
        .or_else(|| s.strip_prefix("bearer "))
        .map(str::trim)
        .filter(|x| !x.is_empty())?;
    Some(t)
}

fn bearer_raw(req: &ServiceRequest) -> Option<&str> {
    bearer_from_authorization_value(
        req.headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok()),
    )
}

/// Middleware на базе [`actix_web::middleware::from_fn`].
pub async fn jwt_posts_middleware(
    req: ServiceRequest,
    next: Next<BoxBody>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    match *req.method() {
        Method::GET | Method::HEAD | Method::OPTIONS => return next.call(req).await,
        _ => {}
    }

    let auth = req.app_data::<Data<AuthService>>().ok_or_else(|| {
        actix_web::error::ErrorInternalServerError("AuthService missing from app data")
    })?;

    let token = bearer_raw(&req)
        .ok_or_else(|| ErrorUnauthorized("missing or malformed Authorization: Bearer header"))?;

    let claims = auth
        .verify_bearer(token)
        .map_err(|_| ErrorUnauthorized("invalid or expired Bearer token"))?;

    req.extensions_mut().insert(claims);
    next.call(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_parses_standard_header() {
        assert_eq!(
            bearer_from_authorization_value(Some("Bearer abc.def.ghi")),
            Some("abc.def.ghi")
        );
    }

    #[test]
    fn bearer_rejects_missing() {
        assert_eq!(bearer_from_authorization_value(None), None);
        assert_eq!(bearer_from_authorization_value(Some("Basic xxx")), None);
    }
}
