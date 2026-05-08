use sqlx::PgPool;

#[derive(Clone)]
pub struct BlogService {
    pub pool: PgPool,
}

impl BlogService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
