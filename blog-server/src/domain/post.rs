//! Пост блога: поля строки таблицы `posts` и тела создания и обновления поста через HTTP.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Строка таблицы `posts` без JOIN (удобно для проверки владельца).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Пост для ответов HTTP/gRPC: вместе с **отображаемым** именем автора из `users.username`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PostPublic {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub author_username: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Тело `POST /api/posts`; `author_id` подставляет сервер из JWT, не из клиента.
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
}

/// Тело `PUT /api/posts/{id}`; проверка владельца выполняется в сервисе.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePostRequest {
    pub title: String,
    pub content: String,
}
