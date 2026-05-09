//! Точка входа сервера: загрузка `.env`, создание пула Postgres, общие сервисы и два слушателя (HTTP+gRPC).

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
use data::{post_repository::PostRepository, user_repository::UserRepository};
use dotenvy::dotenv;
use infrastructure::{
    database::{create_pool, run_migrations},
    jwt::JwtService,
    logging::init_tracing,
};
use presentation::{grpc_service, grpc_service::GrpcBlogService, http_handlers, middleware};
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

    let jwt_arc = Arc::new(JwtService::try_new(&jwt_secret)?);
    let users = UserRepository::new(pool.clone());
    let posts = PostRepository::new(pool.clone());
    let auth_service = Arc::new(AuthService::new(users, jwt_arc));
    let blog_service = Arc::new(BlogService::new(posts));

    let http_addr = format!("0.0.0.0:{http_port}");
    let grpc_addr = format!("0.0.0.0:{grpc_port}").parse()?;

    let http_auth = auth_service.clone();
    let http_blog = blog_service.clone();
    let grpc_blog = GrpcBlogService::new(auth_service.clone(), blog_service.clone());
    let jwt_posts = middleware::jwt_bearer_middleware!(auth_service.clone());

    let http_server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(web::Data::from(http_auth.clone()))
            .app_data(web::Data::from(http_blog.clone()))
            .route("/health", web::get().to(http_handlers::health))
            .service(
                web::scope("/api/auth")
                    .route("/register", web::post().to(http_handlers::register))
                    .route("/login", web::post().to(http_handlers::login)),
            )
            .service(
                web::scope("/api/posts")
                    .route("", web::get().to(http_handlers::list_posts))
                    .route(
                        "",
                        web::post()
                            .wrap(jwt_posts.clone())
                            .to(http_handlers::create_post),
                    )
                    .route("/{id}", web::get().to(http_handlers::get_post))
                    .route(
                        "/{id}",
                        web::put()
                            .wrap(jwt_posts.clone())
                            .to(http_handlers::update_post),
                    )
                    .route(
                        "/{id}",
                        web::delete()
                            .wrap(jwt_posts.clone())
                            .to(http_handlers::delete_post),
                    ),
            )
    })
    .bind(&http_addr)?
    .run();

    let grpc_server = Server::builder()
        .add_service(grpc_service::blog::blog_service_server::BlogServiceServer::new(grpc_blog));

    info!("HTTP listening on {}", http_addr);
    info!("gRPC listening on {}", grpc_port);

    let grpc_task = grpc_server.serve(grpc_addr);
    let (_http_res, _grpc_res) = tokio::join!(http_server, grpc_task);
    Ok(())
}
