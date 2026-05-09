//! REST-конечные точки Actix: JSON в телах, пагинация через строку запроса, Bearer JWT на мутациях.
//!
//! Маршруты собираются в `main`; префикс `/api/posts` задаётся обёрткой `web::scope`.

use std::sync::Arc;

use actix_web::{
    http::StatusCode,
    web::{self, Data, ReqData},
    HttpResponse,
};
use serde::Deserialize;

use crate::application::{auth_service::AuthService, blog_service::BlogService};
use crate::domain::{
    post::{CreatePostRequest, UpdatePostRequest},
    user::{LoginRequest, RegisterRequest},
};
use crate::infrastructure::jwt::Claims;

use super::http_error::domain_error_response;

/// Поля строки запроса для списка постов: ограничение и смещение страницы.
#[derive(Debug, Deserialize)]
pub struct PostsListQuery {
    /// По умолчанию 20, дальше в обработчике жёстко ограничивается сверху.
    #[serde(default = "posts_default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn posts_default_limit() -> i64 {
    20
}

#[derive(serde::Serialize)]
struct AuthResponseBody {
    token: String,
    user: crate::domain::user::User,
}

/// Простая проверка живости процесса (подходит для балансировщиков и smoke-тестов).
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}

/// Создание аккаунта; при успехе отдаёт `201` и токен Bearer для последующих вызовов.
pub async fn register(
    auth: Data<Arc<AuthService>>,
    body: web::Json<RegisterRequest>,
) -> HttpResponse {
    match auth.register(body.into_inner()).await {
        Ok((token, user)) => {
            HttpResponse::build(StatusCode::CREATED).json(AuthResponseBody { token, user })
        }
        Err(e) => domain_error_response(e),
    }
}

/// Проверка пары имя пользователя и пароля; ошибка авторизации всегда `401` через доменную ветку.
pub async fn login(auth: Data<Arc<AuthService>>, body: web::Json<LoginRequest>) -> HttpResponse {
    match auth.login(body.into_inner()).await {
        Ok((token, user)) => HttpResponse::Ok().json(AuthResponseBody { token, user }),
        Err(e) => domain_error_response(e),
    }
}

/// Список постов с пагинацией по `limit`/`offset`; поле `total` отражает размер всей таблицы.
///
/// В gRPC методе списка тот же срез задаёт парой полей страницы: номер страницы (≥ 1) и размер страницы;
/// они превращаются в те же `(limit, offset)` до вызова репозитория.
pub async fn list_posts(
    blog: Data<Arc<BlogService>>,
    query: web::Query<PostsListQuery>,
) -> HttpResponse {
    let lim = query.limit.clamp(1, 100);
    let off = query.offset.max(0);
    match blog.list_posts(lim, off).await {
        Ok((posts, total)) => HttpResponse::Ok().json(serde_json::json!({
            "posts": posts,
            "total": total,
            "limit": lim,
            "offset": off,
        })),
        Err(e) => domain_error_response(e),
    }
}

/// Чтение одного поста по числовому идентификатору из пути сегмента `{id}`.
pub async fn get_post(blog: Data<Arc<BlogService>>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();
    match blog.get_post(id).await {
        Ok(post) => HttpResponse::Ok().json(post),
        Err(e) => domain_error_response(e),
    }
}

/// Создание поста: автор берётся из JWT, который `HttpAuthentication::bearer` кладёт в расширения запроса до хендлера.
pub async fn create_post(
    claims: ReqData<Claims>,
    blog: Data<Arc<BlogService>>,
    body: web::Json<CreatePostRequest>,
) -> HttpResponse {
    match blog.create_post(claims.user_id, body.into_inner()).await {
        Ok(post) => HttpResponse::build(StatusCode::CREATED).json(post),
        Err(e) => domain_error_response(e),
    }
}

/// Обновляет пост только владельцу; JWT проверяется до входа в обработчик, номер строки задаётся путём.
pub async fn update_post(
    claims: ReqData<Claims>,
    blog: Data<Arc<BlogService>>,
    path: web::Path<i64>,
    body: web::Json<UpdatePostRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    match blog
        .update_post(claims.user_id, id, body.into_inner())
        .await
    {
        Ok(post) => HttpResponse::Ok().json(post),
        Err(e) => domain_error_response(e),
    }
}

/// Удаление поста владельцем; успешный результат — ответ без тела с кодом `204`.
pub async fn delete_post(
    claims: ReqData<Claims>,
    blog: Data<Arc<BlogService>>,
    path: web::Path<i64>,
) -> HttpResponse {
    let id = path.into_inner();
    match blog.delete_post(claims.user_id, id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => domain_error_response(e),
    }
}
