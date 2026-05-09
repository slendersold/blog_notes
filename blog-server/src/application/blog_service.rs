//! Операции с постами: проверки владельца на изменение и удалении, выбор списков.

use crate::{
    data::post_repository::PostRepository,
    domain::{
        error::DomainError,
        post::{CreatePostRequest, Post, UpdatePostRequest},
        validation,
    },
};

/// Сервис блога над [`PostRepository`]; не знает о транспортном протоколе (HTTP или gRPC).
#[derive(Clone)]
pub struct BlogService {
    posts: PostRepository,
}

impl BlogService {
    /// Связывает сервис с репозиторием постов.
    pub fn new(posts: PostRepository) -> Self {
        Self { posts }
    }

    /// Создание поста; `author_id` приходит только из JWT, а не из тела клиента.
    pub async fn create_post(
        &self,
        author_id: i64,
        req: CreatePostRequest,
    ) -> Result<Post, DomainError> {
        validation::post_payload(&req.title, &req.content)?;
        self.posts.insert(&req.title, &req.content, author_id).await
    }

    /// Получение одной записи без проверки прав (публичное чтение по ТЗ).
    pub async fn get_post(&self, id: i64) -> Result<Post, DomainError> {
        self.posts.get_by_id(id).await
    }

    /// Изменение чужих постов запрещено — перед обновлением сверяем `author_id`.
    pub async fn update_post(
        &self,
        author_id: i64,
        id: i64,
        req: UpdatePostRequest,
    ) -> Result<Post, DomainError> {
        validation::post_payload(&req.title, &req.content)?;
        match self.posts.get_by_id(id).await {
            Ok(p) if p.author_id == author_id => {
                self.posts
                    .update(id, &req.title, &req.content, author_id)
                    .await
            }
            Ok(_) => Err(DomainError::Forbidden),
            Err(e) => Err(e),
        }
    }

    /// Удаляет только посты того же пользователя через условное удаление репозитория.
    pub async fn delete_post(&self, author_id: i64, id: i64) -> Result<(), DomainError> {
        match self.posts.get_by_id(id).await {
            Ok(p) if p.author_id == author_id => self.posts.delete(id, author_id).await,
            Ok(_) => Err(DomainError::Forbidden),
            Err(e) => Err(e),
        }
    }

    /// Список с пагинацией по `LIMIT`/`OFFSET` и суммой строк для интерфейса пагинации.
    ///
    /// В REST передаём `limit`/`offset` в строке запроса; в gRPC — номер страницы (начиная с единицы) и размер страницы,
    /// которые на уровне репозитория сводятся к `OFFSET = (page - 1) * limit`.
    pub async fn list_posts(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Post>, i64), DomainError> {
        self.posts.page(limit, offset).await
    }
}
