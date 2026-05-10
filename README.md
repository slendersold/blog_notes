# blog_notes

Workspace блог на Rust: REST и gRPC бэкенд блога, общая клиентская библиотека, CLI и браузерный фронт на WebAssembly. Данные — PostgreSQL, аутентификация — JWT (Bearer).

## Контракт входа: email, не username

В учебном ТЗ для `POST /api/auth/login` иногда приведён JSON с полем **`username`**. В этом репозитории **везде согласован вход по `email`** (тот же, что при регистрации) и **`password`**:

- HTTP **`POST /api/auth/login`**, gRPC **`LoginRequest`** в `proto/blog.proto`;
- **blog-cli**: `login --email … --password …`;
- **blog-wasm** и **`index.html`**: форма входа по email.

Регистрация по-прежнему: **`username`**, **`email`**, **`password`**. Если проверяющий ожидает пример с `username` в теле логина — используйте **`email`** или согласуйте отличие явно.

## Архитектура и крейты

| Крейт | Назначение |
|--------|------------|
| **blog-server** | HTTP (Actix), gRPC (Tonic), PostgreSQL (sqlx), JWT, миграции при старте. Общая бизнес-логика в сервисах; HTTP и gRPC делят `AuthService` и `BlogService`. |
| **blog-client** | Библиотека: один API для двух транспортов — `Transport::Http` (reqwest, REST `/api/...`) и `Transport::Grpc` (tonic, код из `proto/blog.proto`). Хранит JWT после `register` / `login`. |
| **blog-cli** | Консольная утилита поверх `blog-client`: подкоманды register, login, CRUD постов, list; токен сохраняется в `.blog_token` в текущей директории. |
| **blog-wasm** | Фронт в браузере: `wasm-bindgen`, HTTP через `gloo-net`, JWT в `localStorage` (ключ `blog_token`). Сборка даёт JS-glue в `blog-wasm/pkg/`. |
| **blog-integration-tests** | Интеграционный сценарий: Postgres в Docker + сервер + пять параллельных CLI; не обязателен для ручного запуска блога. |

Связь по данным: **blog-server** — источник истины в БД. **blog-client** не зависит от сервера как от крейта, только совместим по контрактам API.

В **одном Cargo workspace** пять членов (четыре основных крейта по ТЗ плюс **blog-integration-tests**). По **Cargo** бинарник **blog-cli** зависит от **blog-client**; **blog-wasm** к **blog-client не подключается**: в браузере HTTP идёт через `gloo-net` к REST сервера.

```text
                    ┌─────────────┐
                    │ PostgreSQL  │
                    └──────▲──────┘
                           │
                  ┌────────┴────────┐
                  │   blog-server   │
                  │  HTTP + gRPC    │
                  └────────┬────────┘
                           │
           ┌───────────────┴───────────────┐
           │                               │
      HTTP / gRPC                     HTTP (REST)
           │                               │
    ┌──────┴──────┐                 ┌──────┴──────┐
    │  blog-cli   │                 │ blog-wasm   │
    │  (бинарник) │                 │ (браузер)   │
    └──────┬──────┘                 └─────────────┘
           │
           │  зависимость Cargo: crate blog-cli → blog-client
           ▼
    ┌──────────────┐
    │ blog-client  │
    │ (библиотека) │
    └──────────────┘
```

## Требования

- **Rust** (stable), **Cargo**, цель **wasm32-unknown-unknown** для фронта (`rustup target add wasm32-unknown-unknown`).
- **PostgreSQL 17** (или совместимая версия).
- Для сборки WASM-обвязки: **`wasm-bindgen`** из пакета [wasm-bindgen-cli](https://rustwasm.github.io/wasm-bindgen/) (`cargo install wasm-bindgen-cli`), либо [**wasm-pack**](https://rustwasm.github.io/wasm-pack/).

## Окружение и секреты

1. Скопируйте пример окружения и подставьте секреты:

   ```bash
   cp .env.example .env
   ```

2. **`DATABASE_URL`** — строка подключения к Postgres (см. пример в `.env.example`). Для Docker ниже пользователь/БД совпадают с `docker-compose.yml`.

3. **`JWT_SECRET`** — достаточно длинная случайная строка для подписи JWT (на практике не короче 32 символов). Пример генерации:

   ```bash
   openssl rand -hex 32
   ```

   Вставьте результат в `.env` как значение `JWT_SECRET`.

4. **`HTTP_PORT`** / **`GRPC_PORT`** — порты HTTP и gRPC (по умолчанию `8080` и `50051`).

Файл **`.env`** не коммитьте; в репозитории только **`.env.example`**.

## PostgreSQL

### Вариант A: Docker Compose

Из корня проекта:

```bash
docker compose up -d
```

Поднимется Postgres с пользователем `blog_user`, паролем `blog_pass`, БД `blog_db`, порт **5432** по умолчанию (`PG_HOST_PORT` в `docker-compose.yml` — другой порт, если занят или для изолированного стека интеграционных тестов). Строка подключения может совпадать с `.env.example`.

### Вариант B: локальный Postgres

Создайте пользователя и базу и пропишите их в `DATABASE_URL`.

Миграции применяются **при старте сервера** автоматически.

## Сборка workspace

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
```

При необходимости воспроизводимых сборок закоммитьте **`Cargo.lock`** (сейчас он может быть в `.gitignore` по условиям ТЗ — ориентируйтесь на требования курса и CI).

Артефакты Cargo по умолчанию собираются в **`tmp/target`** (см. `.cargo/config.toml`), каталог **`tmp/`** в git не входит.

---

## Тесты (локально)

| Задача | Команды (из каталога `blog_notes`) |
|--------|-------------------------------------|
| Все юнит- и интеграционные тесты крейтов, **кроме** тяжёлого Docker-сценария | `cargo test --workspace --exclude blog-integration-tests` |
| Полный прогон, включая **blog-integration-tests** (нужны **Docker** и **Compose v2**) | Сначала `cargo build -p blog-cli -p blog-server`, затем `cargo test --workspace` или только `cargo test -p blog-integration-tests --test cli_five_clients_integration` |
| Миграции на живой Postgres (переменная **`DATABASE_URL`**, тест помечен **`#[ignore]`**) | `cargo test -p blog-server --test migrations_integration -- --ignored --nocapture` |

Крейт **blog-integration-tests** в `build.rs` проверяет наличие бинарников `blog-cli` и `blog-server` в каталоге target (в т.ч. `tmp/target`); при отсутствии — сообщение с подсказкой собрать их явно.

---

## CI (GitHub Actions)

В **`.github/workflows/`** настроены проверки:

| Файл | Когда | Содержание |
|------|--------|------------|
| **`ci-main.yml`** | Пуш и **pull request** в `main` / `master`, вручную (**Actions → Run workflow**) | `fmt`, `clippy -D warnings`, `cargo build --workspace`, сборка **blog-cli** + **blog-server**, **`cargo test --workspace`**, тест миграций с Postgres service (`--ignored`), сборка **blog-wasm** под `wasm32-unknown-unknown`. Для сценария **blog-integration-tests** на раннере доступен Docker. |
| **`ci-branch-*.yml`** | Пуш в прочие ветки при изменении соответствующих путей | Ускоренные проверки отдельных крейтов без полного прогона. |

Локально повторить главный сценарий: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --no-deps -- -D warnings`, затем команды из раздела **Тесты**.

Если на GitHub корень репозитория **не** совпадает с каталогом workspace (например workspace в подкаталоге), в workflow нужно задать **`defaults.run.working-directory`** на этот каталог.

---

## Запуск компонентов

Нужны файл **`.env`** в корне `blog_notes` (см. `.env.example`), поднятая БД (**`docker compose up -d`** или свой Postgres).

### Сервер (`blog-server`)

Сервер сам подхватывает `.env` (через `dotenvy`).

```bash
./scripts/run-server.sh
```

Эквивалент: `cargo run --bin blog-server` из каталога `blog_notes`.

Ожидаемо:

- HTTP: `http://127.0.0.1:8080` (или `HTTP_PORT` в `.env`).
- gRPC: порт `50051` (или `GRPC_PORT`).
- Проверка живости: `GET /health`.

Остановка: **Ctrl+C** (корректно гасятся и HTTP, и gRPC).

### Статическая раздача WASM (`index.html` + `blog-wasm/pkg/`)

После `./scripts/build-wasm-web.sh` (или `wasm-pack build --target web` в `blog-wasm/`):

```bash
./scripts/serve-wasm-web.sh
```

По умолчанию порт **8765**; аргументом можно задать другой, например `./scripts/serve-wasm-web.sh 8000`. В браузере откройте напечатанный URL; **`API_BASE`** в `index.html` должен совпадать с HTTP-портом сервера.

### Клиентская библиотека (`blog-client`)

Отдельно не «запускается» — подключается как зависимость. Проверка сборки:

```bash
cargo build -p blog-cli
```

### CLI (`blog-cli`)

Работает из любой директории; ищет `.env` в текущей папке и пишет токен в **`.blog_token`**.

По умолчанию HTTP `http://127.0.0.1:8080`, для gRPC добавьте **`--grpc`** и при необходимости **`--server http://127.0.0.1:50051`**.

Примеры:

```bash
# Регистрация и сохранение JWT в .blog_token
cargo run -p blog-cli -- register \
  --username alice \
  --email alice@example.com \
  --password secret123

# Вход
cargo run -p blog-cli -- login --email alice@example.com --password secret123

# Посты (нужен предыдущий login/register в этой же директории)
cargo run -p blog-cli -- create --title "Привет" --content "Текст поста"
cargo run -p blog-cli -- list --limit 20 --offset 0
cargo run -p blog-cli -- get --id 1
cargo run -p blog-cli -- update --id 1 --title "Новый заголовок" --content "Новый текст"
cargo run -p blog-cli -- delete --id 1

# Тот же сценарий по gRPC
cargo run -p blog-cli -- --grpc register --username bob --email bob@example.com --password secret456
cargo run -p blog-cli -- --grpc login --email bob@example.com --password secret456
cargo run -p blog-cli -- --grpc create --title "gRPC пост" --content "Содержание"
```

### WASM-фронт (`blog-wasm`)

Кратко: сборка артефактов, затем **`./scripts/serve-wasm-web.sh`** (см. выше).

1. Соберите модуль и JS-glue:

   ```bash
   ./scripts/build-wasm-web.sh
   ```

   Нужны `cargo`, цель `wasm32-unknown-unknown` и **`wasm-bindgen`** в `PATH`.

   Альтернатива:

   ```bash
   cd blog-wasm
   wasm-pack build --target web
   ```

2. Запустите раздачу статики из корня `blog_notes` — **`./scripts/serve-wasm-web.sh [порт]`** или вручную `python3 -m http.server` из того же каталога.

3. Откройте в браузере напечатанный URL. **`API_BASE`** в `index.html` (по умолчанию `http://127.0.0.1:8080`) должен указывать на ваш HTTP-бэкенд.

**Кеш браузера:** при странном UI сделайте жёсткое обновление (**Ctrl+Shift+R** / **Cmd+Shift+R**). Регистрация и **вход по email** — см. раздел «Контракт входа» выше.

Сгенерированный каталог **`blog-wasm/pkg/`** обычно не коммитится; после клонирования сборку нужно повторить.

---

## Сценарии проверки для ревью

### curl (HTTP)

Замените при необходимости хост и порт.

```bash
# Регистрация
curl -s -X POST http://127.0.0.1:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"curl_user","email":"curl@example.com","password":"secret123"}'

# Ответ содержит token — подставьте в переменную (для шагов ниже часто достаточно токена из register):
export TOKEN="<jwt из ответа>"

# Отдельный вход пользователя, уже зарегистрированного ранее (в теле — email и password)
curl -s -X POST http://127.0.0.1:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"curl@example.com","password":"secret123"}'

# Создание поста
curl -s -X POST http://127.0.0.1:8080/api/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"title":"Заголовок","content":"Текст"}'

# Список постов (публично)
curl -s "http://127.0.0.1:8080/api/posts?limit=10&offset=0"

# Один пост
curl -s http://127.0.0.1:8080/api/posts/1
```

### CLI

Полный цикл: `register` → **`login` по `--email`** (если новая сессия без `.blog_token`) → `create` → `list` → `get` → `update` → `delete` (примеры в разделе CLI выше).

### Браузер

1. Запущены Postgres, `./scripts/run-server.sh` (или `cargo run --bin blog-server`), собран `blog-wasm/pkg`, статика через **`./scripts/serve-wasm-web.sh`**.
2. При необходимости поправить `API_BASE` в `index.html` под свой бэкенд.
3. Регистрация или **вход по email** — токен в `localStorage` (см. «Контракт входа»).
4. Создать пост; для своих постов доступны изменение и удаление.

---

## Полезное

- Токен CLI: файл **`.blog_token`** в рабочей директории (не коммитится).
- Ошибки и запросы сервера логируются через **tracing** (уровень можно задать переменной окружения, например `RUST_LOG=info`).
- CORS на HTTP настроен пермиссивно для локальной разработки фронта.
- Проверки в CI: раздел **CI (GitHub Actions)** и каталог **`.github/workflows/`**.

## Лицензия

Как в **Cargo.toml** workspace: `MIT OR Apache-2.0`.
