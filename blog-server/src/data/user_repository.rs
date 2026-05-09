//! Вставка и выбор пользователей; нарушения уникальности username/email переводятся в `DomainError`.

use sqlx::PgPool;

use crate::domain::{error::DomainError, user::User};

/// Репозиторий поверх общего пула Postgres; каждый запрос параметризован.
#[derive(Clone)]
pub struct UserRepository {
    pub pool: PgPool,
}

impl UserRepository {
    /// Связывает репозиторий с уже созданным пулом миграций.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Вставляет строку после того, как слой приложения сгенерировал соль и Argon2-хеш.
    ///
    /// При конфликте уникальных индексов по username или email возвращает [`DomainError::UserAlreadyExists`].
    pub async fn insert(
        &self,
        username: &str,
        email: &str,
        password_salt: &str,
        password_hash: &str,
    ) -> Result<User, DomainError> {
        let q = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_salt, password_hash)
            VALUES ($1, $2, $3, $4)
            RETURNING id, username, email, password_salt, password_hash, created_at
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_salt)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await;

        Self::map_insert_err(q).await
    }

    async fn map_insert_err(r: sqlx::Result<User>) -> Result<User, DomainError> {
        match r {
            Ok(u) => Ok(u),
            Err(sqlx::Error::Database(d)) => {
                // Код Postgres 23505 — нарушение уникального ограничения (username или email).
                if d.code().as_deref() == Some("23505") {
                    Err(DomainError::UserAlreadyExists)
                } else {
                    Err(DomainError::DataBaseInternal(d.to_string()))
                }
            }
            Err(e) => Err(DomainError::DataBaseInternal(e.to_string())),
        }
    }

    /// Возвращает пользователя при точном совпадении имени входа или `UserNotFound`, если строки нет.
    pub async fn get_by_username(&self, username: &str) -> Result<User, DomainError> {
        sqlx::query_as::<_, User>(
            r#"SELECT id, username, email, password_salt, password_hash, created_at FROM users WHERE username = $1"#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::DataBaseInternal(e.to_string()))?
        .ok_or(DomainError::UserNotFound)
    }
}
