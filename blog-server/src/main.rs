mod application;
mod data;
mod domain;
mod infrastructure;
mod presentation;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use anyhow::Context;
use application::{auth_service::AuthService, blog_service::BlogService};
use dotenvy::dotenv;
use infrastructure::{
    database::{create_pool, run_migrations},
    jwt::JwtService,
    logging::init_tracing,
};
use presentation::{grpc_service, grpc_service::GrpcBlogService, http_handlers};
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    init_tracing();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
    let jwt_secret = std::env::var("JWT_SECRET").context("JWT_SECRET is not set")?;
    let http_port = std::env::var("HTTP_PORT").unwrap_or_else(|_| "8080".to_string());
    let grpc_port = std::env::var("GRPC_PORT").unwrap_or_else(|_| "50051".to_string());

    let pool = create_pool(&database_url).await?;
    run_migrations(&pool).await?;

    let jwt_service = Arc::new(JwtService::new(&jwt_secret));
    let auth_service = Arc::new(AuthService::new(pool.clone(), jwt_service.clone()));
    let blog_service = Arc::new(BlogService::new(pool.clone()));

    let http_addr = format!("0.0.0.0:{http_port}");
    let grpc_addr = format!("0.0.0.0:{grpc_port}").parse()?;

    let http_auth_service = auth_service.clone();
    let http_blog_service = blog_service.clone();
    let http_server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(web::Data::from(http_auth_service.clone()))
            .app_data(web::Data::from(http_blog_service.clone()))
            .route("/health", web::get().to(http_handlers::health))
            .route("/api/posts", web::get().to(http_handlers::list_posts))
    })
    .bind(&http_addr)?
    .run();

    let grpc_blog_service = GrpcBlogService::new(auth_service, blog_service);
    let grpc_server = Server::builder()
        .add_service(
            grpc_service::blog::blog_service_server::BlogServiceServer::new(grpc_blog_service),
        )
        .serve(grpc_addr);

    info!("HTTP listening on {}", http_addr);
    info!("gRPC listening on {}", grpc_port);

    let (_http_res, _grpc_res) = tokio::join!(http_server, grpc_server);
    Ok(())
}
