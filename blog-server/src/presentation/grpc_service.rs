use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::application::{auth_service::AuthService, blog_service::BlogService};

pub mod blog {
    tonic::include_proto!("blog");
}

#[derive(Clone)]
pub struct GrpcBlogService {
    #[allow(dead_code)]
    auth_service: Arc<AuthService>,
    #[allow(dead_code)]
    blog_service: Arc<BlogService>,
}

impl GrpcBlogService {
    pub fn new(auth_service: Arc<AuthService>, blog_service: Arc<BlogService>) -> Self {
        Self {
            auth_service,
            blog_service,
        }
    }
}

#[tonic::async_trait]
impl blog::blog_service_server::BlogService for GrpcBlogService {
    async fn register(
        &self,
        _request: Request<blog::RegisterRequest>,
    ) -> Result<Response<blog::AuthResponse>, Status> {
        Err(Status::unimplemented("register is not implemented yet"))
    }

    async fn login(
        &self,
        _request: Request<blog::LoginRequest>,
    ) -> Result<Response<blog::AuthResponse>, Status> {
        Err(Status::unimplemented("login is not implemented yet"))
    }

    async fn create_post(
        &self,
        _request: Request<blog::CreatePostRequest>,
    ) -> Result<Response<blog::PostResponse>, Status> {
        Err(Status::unimplemented("create_post is not implemented yet"))
    }

    async fn get_post(
        &self,
        _request: Request<blog::GetPostRequest>,
    ) -> Result<Response<blog::PostResponse>, Status> {
        Err(Status::unimplemented("get_post is not implemented yet"))
    }

    async fn update_post(
        &self,
        _request: Request<blog::UpdatePostRequest>,
    ) -> Result<Response<blog::PostResponse>, Status> {
        Err(Status::unimplemented("update_post is not implemented yet"))
    }

    async fn delete_post(
        &self,
        _request: Request<blog::DeletePostRequest>,
    ) -> Result<Response<blog::DeletePostResponse>, Status> {
        Err(Status::unimplemented("delete_post is not implemented yet"))
    }

    async fn list_posts(
        &self,
        _request: Request<blog::ListPostsRequest>,
    ) -> Result<Response<blog::ListPostsResponse>, Status> {
        Err(Status::unimplemented("list_posts is not implemented yet"))
    }
}
