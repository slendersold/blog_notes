//! Пользователь: поля для хранения в БД и структуры тел REST-запросов регистрации и входа.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Запись пользователя в Postgres; `password_salt` дублирует соль из PHC для прозрачности в SQL, проверка — по `password_hash`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    /// Соль Argon2 в открытом виде (хранится рядом с хешем — нормальная практика, секретность даёт сам пароль).
    #[serde(skip_serializing)]
    pub password_salt: String,
    /// Полная PHC-строка Argon2; при логине парсится и сверяется вместе с солью внутри записи.
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Тело `POST /api/auth/register`; поля затем переходят в репозиторий после хеширования пароля.
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// Тело `POST /api/auth/login`; только идентификатор входа и открытый пароль для проверки.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}
