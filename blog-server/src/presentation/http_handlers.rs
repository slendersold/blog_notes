use std::sync::Arc;

use actix_web::{web, HttpResponse, Responder};

use crate::application::{auth_service::AuthService, blog_service::BlogService};

pub async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}

pub async fn list_posts(
    _auth: web::Data<Arc<AuthService>>,
    _blog: web::Data<Arc<BlogService>>,
) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "posts": [],
        "total": 0,
        "limit": 20,
        "offset": 0
    }))
}
