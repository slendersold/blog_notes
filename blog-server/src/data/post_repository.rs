use sqlx::PgPool;

#[derive(Clone)]
pub struct PostRepository {
    pub pool: PgPool,
}

impl PostRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
