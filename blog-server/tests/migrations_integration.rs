//! Интеграция с реальным Postgres: миграции накатываются и повторный прогон не падает.
//!
//! По умолчанию тест отмечен `#[ignore]`, чтобы локальный `cargo test --lib` не требовал БД.
//! В CI на ветке `main` задаётся `DATABASE_URL` и запуск включают флаг `--ignored`.

use sqlx::postgres::PgPoolOptions;

#[tokio::test]
#[ignore = "нужны переменная DATABASE_URL и запущенный PostgreSQL"]
async fn migrations_apply_twice_without_error() {
    let url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set when running ignored test");

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("postgres connect failed");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("first migration run failed");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("second migration run failed (идемпотентность)");
}
