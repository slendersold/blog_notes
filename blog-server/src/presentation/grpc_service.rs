//! gRPC-адаптер: преобразует Protobuf сообщения из `proto/blog.proto` в вызовы прикладных сервисов.
//!
//! Для мутаций с политикой «только автор» читает Bearer-токен из метаданных `authorization` так же,
//! как HTTP слой через заголовки.
#![allow(clippy::result_large_err)]

use std::sync::Arc;

use tonic::{metadata::MetadataMap, Request, Response, Status};

use crate::application::{auth_service::AuthService, blog_service::BlogService};
use crate::domain::{
    error::DomainError,
    post::{CreatePostRequest as DCreatePost, UpdatePostRequest as DUpdatePost},
    user::{LoginRequest as DLogin, RegisterRequest as DRegister},
};
use crate::infrastructure::jwt::Claims;

/// Сгенерированный tonic-модуль; имя файла задаёт префикс пакета `blog`.
pub mod blog {
    tonic::include_proto!("blog");
}

/// Сводит доменную ошибку в стандартный `tonic::Status` для машиночитаемого ответа клиентам gRPC.
fn grpc_status(err: DomainError) -> Status {
    match err {
        DomainError::Validation(s) => Status::invalid_argument(s),
        DomainError::Unauthorized => Status::unauthenticated("unauthorized"),
        DomainError::InvalidCredentials => Status::unauthenticated("invalid credentials"),
        DomainError::Forbidden => Status::permission_denied("forbidden"),
        DomainError::UserNotFound => Status::not_found("user not found"),
        DomainError::PostNotFound => Status::not_found("post not found"),
        DomainError::UserAlreadyExists => Status::already_exists("user already exists"),
        DomainError::DataBaseInternal(m) => Status::internal(m),
    }
}

/// Достаёт сырую строку токена из метаданных и ожидает префикс `Bearer` (или `bearer`).
fn extract_bearer(meta: &MetadataMap) -> Result<String, Status> {
    let hv = meta
        .get("authorization")
        .ok_or_else(|| Status::unauthenticated("missing Authorization"))?;
    let s = hv
        .to_str()
        .map_err(|_| Status::invalid_argument("authorization must be ascii"))?;
    let token = if let Some(t) = s.strip_prefix("Bearer ") {
        t.trim()
    } else if let Some(t) = s.strip_prefix("bearer ") {
        t.trim()
    } else {
        return Err(Status::invalid_argument("expected Bearer token"));
    };
    Ok(token.to_string())
}

fn user_pb(u: &crate::domain::user::User) -> blog::User {
    blog::User {
        id: u.id,
        username: u.username.clone(),
        email: u.email.clone(),
        created_at: u.created_at.to_rfc3339(),
    }
}

fn post_pb(p: &crate::domain::post::PostPublic) -> blog::Post {
    blog::Post {
        id: p.id,
        title: p.title.clone(),
        content: p.content.clone(),
        author_id: p.author_id,
        created_at: p.created_at.to_rfc3339(),
        updated_at: p.updated_at.to_rfc3339(),
        author_username: p.author_username.clone(),
    }
}

/// Тонкая обёртка вокруг `Arc` сервисов; живёт как long-lived объект в tonic `Router`.
#[derive(Clone)]
pub struct GrpcBlogService {
    auth_service: Arc<AuthService>,
    blog_service: Arc<BlogService>,
}

impl GrpcBlogService {
    /// Связывает gRPC транспорт уже сконструированными сервисами приложений.
    pub fn new(auth_service: Arc<AuthService>, blog_service: Arc<BlogService>) -> Self {
        Self {
            auth_service,
            blog_service,
        }
    }

    /// Извлекает claims из Bearer-токена в метаданных одного входящего gRPC сообщения.
    fn authorize(&self, meta: &MetadataMap) -> Result<Claims, Status> {
        let tok = extract_bearer(meta)?;
        self.auth_service.verify_bearer(&tok).map_err(grpc_status)
    }
}

#[tonic::async_trait]
impl blog::blog_service_server::BlogService for GrpcBlogService {
    async fn register(
        &self,
        req: Request<blog::RegisterRequest>,
    ) -> Result<Response<blog::AuthResponse>, Status> {
        let inner = req.into_inner();
        let dto = DRegister {
            username: inner.username,
            email: inner.email,
            password: inner.password,
        };
        match self.auth_service.register(dto).await {
            Ok((token, user)) => Ok(Response::new(blog::AuthResponse {
                token,
                user: Some(user_pb(&user)),
            })),
            Err(e) => Err(grpc_status(e)),
        }
    }

    async fn login(
        &self,
        req: Request<blog::LoginRequest>,
    ) -> Result<Response<blog::AuthResponse>, Status> {
        let inner = req.into_inner();
        let dto = DLogin {
            email: inner.email,
            password: inner.password,
        };
        match self.auth_service.login(dto).await {
            Ok((token, user)) => Ok(Response::new(blog::AuthResponse {
                token,
                user: Some(user_pb(&user)),
            })),
            Err(e) => Err(grpc_status(e)),
        }
    }

    async fn create_post(
        &self,
        req: Request<blog::CreatePostRequest>,
    ) -> Result<Response<blog::PostResponse>, Status> {
        let md = req.metadata().clone();
        let claims = self.authorize(&md)?;
        let inner = req.into_inner();
        let dto = DCreatePost {
            title: inner.title,
            content: inner.content,
        };
        match self.blog_service.create_post(claims.user_id, dto).await {
            Ok(p) => Ok(Response::new(blog::PostResponse {
                post: Some(post_pb(&p)),
            })),
            Err(e) => Err(grpc_status(e)),
        }
    }

    async fn get_post(
        &self,
        req: Request<blog::GetPostRequest>,
    ) -> Result<Response<blog::PostResponse>, Status> {
        let inner = req.into_inner();
        match self.blog_service.get_post(inner.id).await {
            Ok(p) => Ok(Response::new(blog::PostResponse {
                post: Some(post_pb(&p)),
            })),
            Err(e) => Err(grpc_status(e)),
        }
    }

    async fn update_post(
        &self,
        req: Request<blog::UpdatePostRequest>,
    ) -> Result<Response<blog::PostResponse>, Status> {
        let md = req.metadata().clone();
        let claims = self.authorize(&md)?;
        let inner = req.into_inner();
        let dto = DUpdatePost {
            title: inner.title,
            content: inner.content,
        };
        match self
            .blog_service
            .update_post(claims.user_id, inner.id, dto)
            .await
        {
            Ok(p) => Ok(Response::new(blog::PostResponse {
                post: Some(post_pb(&p)),
            })),
            Err(e) => Err(grpc_status(e)),
        }
    }

    async fn delete_post(
        &self,
        req: Request<blog::DeletePostRequest>,
    ) -> Result<Response<blog::DeletePostResponse>, Status> {
        let md = req.metadata().clone();
        let claims = self.authorize(&md)?;
        let inner = req.into_inner();
        match self
            .blog_service
            .delete_post(claims.user_id, inner.id)
            .await
        {
            Ok(()) => Ok(Response::new(blog::DeletePostResponse { success: true })),
            Err(e) => Err(grpc_status(e)),
        }
    }

    // Пагинация через page (≥1) и limit; совпадает с REST-срезом limit/offset после пересчёта offset.
    async fn list_posts(
        &self,
        req: Request<blog::ListPostsRequest>,
    ) -> Result<Response<blog::ListPostsResponse>, Status> {
        let inner = req.into_inner();
        let mut page = inner.page;
        if page < 1 {
            page = 1;
        }
        let mut limit = inner.limit;
        if limit <= 0 {
            limit = 20;
        }
        let lim = (limit as i64).clamp(1, 100);
        let page_i = page as i64;
        let off = (page_i - 1) * lim;
        match self.blog_service.list_posts(lim, off).await {
            Ok((posts, total)) => {
                let pb_posts: Vec<_> = posts.iter().map(post_pb).collect();
                let total_i32 = i32::try_from(total).unwrap_or(i32::MAX);
                Ok(Response::new(blog::ListPostsResponse {
                    posts: pb_posts,
                    total: total_i32,
                }))
            }
            Err(e) => Err(grpc_status(e)),
        }
    }
}
