//! Пул соединений к Postgres и автоматический прогон миграций `sqlx` при старте процесса.

use sqlx::{postgres::PgPoolOptions, PgPool};

/// Создаёт пул с ограничением параллельных соединений (значение задано в ТЗ по смыслу «не более пяти»).
///
/// # Входы
///
/// `database_url` — строка `DATABASE_URL`, например postgres://...
///
/// # Результат
///
/// Открытый пул или ошибка первого же неудачного подключения.
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Прогоняет все файлы из каталога `migrations` относительно манифеста crate.
///
/// После успешного вызова схема БД содержит таблицы `users` и `posts`.
pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
