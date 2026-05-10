//! Операции с постами: проверки владельца на изменение и удалении, выбор списков.

use crate::{
    data::post_repository::PostRepository,
    domain::{
        error::DomainError,
        post::{CreatePostRequest, PostPublic, UpdatePostRequest},
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

    /// Создание поста; `author_id` из JWT.
    pub async fn create_post(
        &self,
        author_id: i64,
        req: CreatePostRequest,
    ) -> Result<PostPublic, DomainError> {
        validation::post_payload(&req.title, &req.content)?;
        self.posts.insert(&req.title, &req.content, author_id).await
    }

    /// Публичное чтение с `author_username`.
    pub async fn get_post(&self, id: i64) -> Result<PostPublic, DomainError> {
        self.posts.get_public_by_id(id).await
    }

    /// Обновляет только свой пост.
    pub async fn update_post(
        &self,
        author_id: i64,
        id: i64,
        req: UpdatePostRequest,
    ) -> Result<PostPublic, DomainError> {
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

    pub async fn delete_post(&self, author_id: i64, id: i64) -> Result<(), DomainError> {
        match self.posts.get_by_id(id).await {
            Ok(p) if p.author_id == author_id => self.posts.delete(id, author_id).await,
            Ok(_) => Err(DomainError::Forbidden),
            Err(e) => Err(e),
        }
    }

    pub async fn list_posts(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<PostPublic>, i64), DomainError> {
        self.posts.page(limit, offset).await
    }
}
