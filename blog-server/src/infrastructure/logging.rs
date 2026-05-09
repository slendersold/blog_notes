//! Включает вывод трасс через `tracing` с фильтром из `RUST_LOG` или значением по умолчанию.

use tracing_subscriber::{fmt, EnvFilter};

/// Поднимает подписчик `tracing`; вызывается один раз из `main`; повторный вызов может быть проигнорирован ядром.
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,blog_server=info"));
    let _ignored = fmt().with_env_filter(filter).try_init();
}
