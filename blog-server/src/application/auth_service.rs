use std::sync::Arc;

use sqlx::PgPool;

use crate::infrastructure::jwt::JwtService;

#[derive(Clone)]
pub struct AuthService {
    pub pool: PgPool,
    pub jwt: Arc<JwtService>,
}

impl AuthService {
    pub fn new(pool: PgPool, jwt: Arc<JwtService>) -> Self {
        Self { pool, jwt }
    }
}
