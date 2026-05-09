//! HTTP middleware для повторного использования Bearer JWT между маршрутами с одинаковой политикой.
//!
//! `HttpAuthentication::bearer` читает заголовок до входа в обработчик; при успешной верификации
//! объект [`Claims`](crate::infrastructure::jwt::Claims) кладётся в [`actix_web::HttpMessage::extensions`],
//! откуда его забирает `ReqData<Claims>` уже в теле функции.
//!
//! Уровень средств Rust не даёт описать тип замыкания в сигнатуре функции; поэтому фабрика оформлена макросом,
//! сохраняя единственный литерал замыкания и корректный `.clone()` в `HttpServer::new`.

/// Собирает `HttpAuthentication` с проверкой JWT через [`crate::application::auth_service::AuthService`].
///
/// Аргумент: `Arc<AuthService>` (например `auth_service.clone()` из `main`). Результат можно клонировать
/// и навешивать только на нужные методы постов — чтение остаётся без заголовка.
macro_rules! jwt_bearer_middleware {
    ($auth:expr) => {{
        use actix_web::dev::ServiceRequest;
        use actix_web::HttpMessage;
        let auth: std::sync::Arc<crate::application::auth_service::AuthService> = $auth;

        actix_web_httpauth::middleware::HttpAuthentication::bearer(
            move |req: ServiceRequest,
                  credentials: actix_web_httpauth::extractors::bearer::BearerAuth| {
                let svc = auth.clone();
                async move {
                    match svc.verify_bearer(credentials.token()) {
                        Ok(claims) => {
                            req.extensions_mut().insert(claims);
                            Ok(req)
                        }
                        Err(_) => Err((
                            actix_web::error::ErrorUnauthorized("invalid or expired Bearer token"),
                            req,
                        )),
                    }
                }
            },
        )
    }};
}

pub(crate) use jwt_bearer_middleware;
