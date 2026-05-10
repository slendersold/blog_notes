//! CRUD по таблице `posts` и подсчёт общего числа записей для пагинации.

use sqlx::PgPool;

use crate::domain::{error::DomainError, post::{Post, PostPublic}};

/// Репозиторий постов без бизнес-правил авторства — проверку владельца делает сервис.
#[derive(Clone)]
pub struct PostRepository {
    pub pool: PgPool,
}

impl PostRepository {
    /// Подключение к уже настроенному пулу.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Добавляет пост; в ответе сразу **имя автора** через подзапрос к `users`.
    pub async fn insert(
        &self,
        title: &str,
        content: &str,
        author_id: i64,
    ) -> Result<PostPublic, DomainError> {
        sqlx::query_as::<_, PostPublic>(
            r#"
            INSERT INTO posts (title, content, author_id)
            VALUES ($1, $2, $3)
            RETURNING
                id,
                title,
                content,
                author_id,
                created_at,
                updated_at,
                (SELECT u.username FROM users u WHERE u.id = author_id) AS author_username
            "#,
        )
        .bind(title)
        .bind(content)
        .bind(author_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::DataBaseInternal(e.to_string()))
    }

    /// Строка поста без JOIN (удобно только для проверки `author_id`).
    pub async fn get_by_id(&self, id: i64) -> Result<Post, DomainError> {
        sqlx::query_as::<_, Post>(
            r#"SELECT id, title, content, author_id, created_at, updated_at FROM posts WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::DataBaseInternal(e.to_string()))?
        .ok_or(DomainError::PostNotFound)
    }

    /// Публичное чтение с именем автора.
    pub async fn get_public_by_id(&self, id: i64) -> Result<PostPublic, DomainError> {
        sqlx::query_as::<_, PostPublic>(
            r#"
            SELECT p.id, p.title, p.content, p.author_id,
                   u.username AS author_username,
                   p.created_at, p.updated_at
              FROM posts p
             INNER JOIN users u ON u.id = p.author_id
             WHERE p.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::DataBaseInternal(e.to_string()))?
        .ok_or(DomainError::PostNotFound)
    }

    /// Обновляет пост владельцем и возвращает публичное представление с именем автора.
    pub async fn update(
        &self,
        id: i64,
        title: &str,
        content: &str,
        author_id: i64,
    ) -> Result<PostPublic, DomainError> {
        let res = sqlx::query_as::<_, PostPublic>(
            r#"
            UPDATE posts AS p
               SET title = $2, content = $3, updated_at = NOW()
             WHERE p.id = $1 AND p.author_id = $4
             RETURNING
                p.id,
                p.title,
                p.content,
                p.author_id,
                p.created_at,
                p.updated_at,
                (SELECT u.username FROM users u WHERE u.id = p.author_id) AS author_username
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(content)
        .bind(author_id)
        .fetch_optional(&self.pool)
        .await;

        Self::must_one_public(res, DomainError::PostNotFound).await
    }

    /// Удаляет пост только если автор совпадает; иначе `PostNotFound` (скрывает чужие id).
    pub async fn delete(&self, id: i64, author_id: i64) -> Result<(), DomainError> {
        let r = sqlx::query(r#"DELETE FROM posts WHERE id = $1 AND author_id = $2"#)
            .bind(id)
            .bind(author_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::DataBaseInternal(e.to_string()))?;

        if r.rows_affected() == 0 {
            Err(DomainError::PostNotFound)
        } else {
            Ok(())
        }
    }

    /// Страница списка: новее сверху, с `author_username`.
    pub async fn page(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<PostPublic>, i64), DomainError> {
        let rows = sqlx::query_as::<_, PostPublic>(
            r#"
            SELECT p.id, p.title, p.content, p.author_id,
                   u.username AS author_username,
                   p.created_at, p.updated_at
              FROM posts p
             INNER JOIN users u ON u.id = p.author_id
             ORDER BY p.created_at DESC
             LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::DataBaseInternal(e.to_string()))?;

        let total: i64 = sqlx::query_scalar(r#"SELECT COUNT(*) FROM posts"#)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::DataBaseInternal(e.to_string()))?;

        Ok((rows, total))
    }

    async fn must_one_public(
        attempt: sqlx::Result<Option<PostPublic>>,
        missing: DomainError,
    ) -> Result<PostPublic, DomainError> {
        match attempt {
            Err(e) => Err(DomainError::DataBaseInternal(e.to_string())),
            Ok(Some(p)) => Ok(p),
            Ok(None) => Err(missing),
        }
    }
}
