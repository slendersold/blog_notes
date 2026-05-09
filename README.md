# blog_notes

Проект **модуля 3**: блог на Rust (workspace: сервер, клиент-библиотека, CLI, WASM).

## Быстрый старт (после реализации)

- PostgreSQL, переменные окружения (`DATABASE_URL`, `JWT_SECRET`).
- `cargo run -p blog-server`, CLI `cargo run -p blog-cli`, фронт: `wasm-pack build -p blog-wasm --target web`.

## Локальная разработка

```bash
cargo build --workspace
cargo test --workspace
```
