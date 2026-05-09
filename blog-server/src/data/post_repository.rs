//! CRUD по таблице `posts` и подсчёт общего числа записей для пагинации.

use sqlx::PgPool;

use crate::domain::{error::DomainError, post::Post};

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

    /// Добавляет пост и возвращает полную строку с автогенерируемыми полями.
    pub async fn insert(
        &self,
        title: &str,
        content: &str,
        author_id: i64,
    ) -> Result<Post, DomainError> {
        sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (title, content, author_id)
            VALUES ($1, $2, $3)
            RETURNING id, title, content, author_id, created_at, updated_at
            "#,
        )
        .bind(title)
        .bind(content)
        .bind(author_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::DataBaseInternal(e.to_string()))
    }

    /// Возвращает пост по ключу или `PostNotFound`.
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

    /// Обновляет заголовок и текст; считает затронутые строки чтобы отличить «не найдено».
    pub async fn update(
        &self,
        id: i64,
        title: &str,
        content: &str,
        author_id: i64,
    ) -> Result<Post, DomainError> {
        let res = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts
               SET title = $2, content = $3, updated_at = NOW()
             WHERE id = $1 AND author_id = $4
             RETURNING id, title, content, author_id, created_at, updated_at
             "#,
        )
        .bind(id)
        .bind(title)
        .bind(content)
        .bind(author_id)
        .fetch_optional(&self.pool)
        .await;

        Self::must_one(res, DomainError::PostNotFound).await
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

    /// Список страницы упорядочен по времени создания новее сверху.
    ///
    /// Вторая составляющая — общее число строк в таблице (для клиентской пагинации).
    pub async fn page(&self, limit: i64, offset: i64) -> Result<(Vec<Post>, i64), DomainError> {
        let rows = sqlx::query_as::<_, Post>(
            r#"
            SELECT id, title, content, author_id, created_at, updated_at
              FROM posts
             ORDER BY created_at DESC
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

    async fn must_one(
        attempt: sqlx::Result<Option<Post>>,
        missing: DomainError,
    ) -> Result<Post, DomainError> {
        match attempt {
            Err(e) => Err(DomainError::DataBaseInternal(e.to_string())),
            Ok(Some(p)) => Ok(p),
            Ok(None) => Err(missing),
        }
    }
}
