# mod3-blog

Проект **модуля 3**: блог на Rust (workspace: сервер, клиент-библиотека, CLI, WASM).

Полное ТЗ и чек-листы — в материалах курса (модуль 3, проект блога). При необходимости скопируйте `task3.md` / `task3-plan.md` из репозитория с домашними заданиями рядом с этим проектом.

## Быстрый старт (после реализации)

- PostgreSQL, переменные окружения (`DATABASE_URL`, `JWT_SECRET`).
- `cargo run -p blog-server`, CLI `cargo run -p blog-cli`, фронт: `wasm-pack build -p blog-wasm --target web`.

## Локальная разработка

```bash
cargo build --workspace
cargo test --workspace
```
