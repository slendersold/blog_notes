//! Обёртка над сгенерированным tonic-клиентом: метаданные `Authorization: Bearer` и приведение сообщений proto к доменным типам клиента.

use tonic::Request;

use crate::blog_pb::blog_service_client::BlogServiceClient;
use crate::blog_pb::{
    CreatePostRequest, DeletePostRequest, GetPostRequest, ListPostsRequest,
    LoginRequest as PbLogin, RegisterRequest as PbRegister, UpdatePostRequest,
};
use crate::error::BlogClientError;
use crate::{AuthResponse, ListPostsOutcome, Post, User};

fn map_user(pb: crate::blog_pb::User) -> Result<User, BlogClientError> {
    let dt = pb
        .created_at
        .parse()
        .map_err(|_| BlogClientError::InvalidRequest {
            message: format!("invalid proto User.created_at {:?}", pb.created_at),
        })?;
    Ok(User {
        id: pb.id,
        username: pb.username,
        email: pb.email,
        created_at: dt,
    })
}

fn map_post(pb: crate::blog_pb::Post) -> Result<Post, BlogClientError> {
    Ok(Post {
        id: pb.id,
        title: pb.title,
        content: pb.content,
        author_id: pb.author_id,
        created_at: pb
            .created_at
            .parse()
            .map_err(|_| BlogClientError::InvalidRequest {
                message: format!("invalid proto Post.created_at {:?}", pb.created_at),
            })?,
        updated_at: pb
            .updated_at
            .parse()
            .map_err(|_| BlogClientError::InvalidRequest {
                message: format!("invalid proto Post.updated_at {:?}", pb.updated_at),
            })?,
    })
}

fn apply_bearer<T>(req: &mut Request<T>, token: &str) -> Result<(), BlogClientError> {
    req.metadata_mut().insert(
        "authorization",
        format!("Bearer {token}")
            .parse()
            .map_err(|_| BlogClientError::InvalidRequest {
                message: "illegal Bearer token encoding".into(),
            })?,
    );
    Ok(())
}

pub async fn register(
    client: &mut BlogServiceClient<tonic::transport::Channel>,
    username: &str,
    email: &str,
    password: &str,
) -> Result<AuthResponse, BlogClientError> {
    let inner = client
        .register(PbRegister {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
        })
        .await
        .map_err(BlogClientError::from_grpc_status)?
        .into_inner();

    let user_pb = inner.user.ok_or_else(|| BlogClientError::InvalidRequest {
        message: "AuthResponse missing user".into(),
    })?;

    Ok(AuthResponse {
        token: inner.token,
        user: map_user(user_pb)?,
    })
}

pub async fn login(
    client: &mut BlogServiceClient<tonic::transport::Channel>,
    username: &str,
    password: &str,
) -> Result<AuthResponse, BlogClientError> {
    let inner = client
        .login(PbLogin {
            username: username.to_string(),
            password: password.to_string(),
        })
        .await
        .map_err(BlogClientError::from_grpc_status)?
        .into_inner();

    let user_pb = inner.user.ok_or_else(|| BlogClientError::InvalidRequest {
        message: "AuthResponse missing user".into(),
    })?;

    Ok(AuthResponse {
        token: inner.token,
        user: map_user(user_pb)?,
    })
}

pub async fn create_post(
    client: &mut BlogServiceClient<tonic::transport::Channel>,
    token: &str,
    title: &str,
    content: &str,
) -> Result<Post, BlogClientError> {
    let mut req = Request::new(CreatePostRequest {
        title: title.to_string(),
        content: content.to_string(),
    });
    apply_bearer(&mut req, token)?;

    let post = client
        .create_post(req)
        .await
        .map_err(BlogClientError::from_grpc_status)?
        .into_inner()
        .post
        .ok_or_else(|| BlogClientError::InvalidRequest {
            message: "PostResponse missing post".into(),
        })?;
    map_post(post)
}

pub async fn get_post(
    client: &mut BlogServiceClient<tonic::transport::Channel>,
    id: i64,
) -> Result<Post, BlogClientError> {
    let post = client
        .get_post(GetPostRequest { id })
        .await
        .map_err(BlogClientError::from_grpc_status)?
        .into_inner()
        .post
        .ok_or_else(|| BlogClientError::InvalidRequest {
            message: "PostResponse missing post".into(),
        })?;
    map_post(post)
}

pub async fn update_post(
    client: &mut BlogServiceClient<tonic::transport::Channel>,
    token: &str,
    id: i64,
    title: &str,
    content: &str,
) -> Result<Post, BlogClientError> {
    let mut req = Request::new(UpdatePostRequest {
        id,
        title: title.to_string(),
        content: content.to_string(),
    });
    apply_bearer(&mut req, token)?;

    let post = client
        .update_post(req)
        .await
        .map_err(BlogClientError::from_grpc_status)?
        .into_inner()
        .post
        .ok_or_else(|| BlogClientError::InvalidRequest {
            message: "PostResponse missing post".into(),
        })?;
    map_post(post)
}

pub async fn delete_post(
    client: &mut BlogServiceClient<tonic::transport::Channel>,
    token: &str,
    id: i64,
) -> Result<(), BlogClientError> {
    let mut req = Request::new(DeletePostRequest { id });
    apply_bearer(&mut req, token)?;

    client
        .delete_post(req)
        .await
        .map_err(BlogClientError::from_grpc_status)?;
    Ok(())
}

/// Сводит пару REST `limit`/`offset` к полям страницы gRPC так же, как на сервере (`page` начиная с 1).
pub async fn list_posts(
    client: &mut BlogServiceClient<tonic::transport::Channel>,
    limit: i64,
    offset: i64,
) -> Result<ListPostsOutcome, BlogClientError> {
    let lim_raw = limit.clamp(1, 100);
    let page_i64 = (offset.div_euclid(lim_raw) + 1).max(1);

    let page_i32 = i32::try_from(page_i64).map_err(|_| BlogClientError::InvalidRequest {
        message: "page out of range".into(),
    })?;
    let limit_i32 = i32::try_from(lim_raw).map_err(|_| BlogClientError::InvalidRequest {
        message: "limit out of range".into(),
    })?;

    let inner = client
        .list_posts(ListPostsRequest {
            page: page_i32,
            limit: limit_i32,
        })
        .await
        .map_err(BlogClientError::from_grpc_status)?
        .into_inner();

    let posts = inner
        .posts
        .into_iter()
        .map(map_post)
        .collect::<Result<Vec<_>, _>>()?;

    let effective_offset = ((i64::from(page_i32)) - 1) * lim_raw;

    Ok(ListPostsOutcome {
        posts,
        total: i64::from(inner.total),
        limit: lim_raw,
        offset: effective_offset.max(0),
    })
}
