# blog_notes

Workspace блог на Rust: REST и gRPC бэкенд блога, общая клиентская библиотека, CLI и браузерный фронт на WebAssembly. Данные — PostgreSQL, аутентификация — JWT (Bearer).

## Архитектура и крейты

| Крейт | Назначение |
|--------|------------|
| **blog-server** | HTTP (Actix), gRPC (Tonic), PostgreSQL (sqlx), JWT, миграции при старте. Общая бизнес-логика в сервисах; HTTP и gRPC делят `AuthService` и `BlogService`. |
| **blog-client** | Библиотека: один API для двух транспортов — `Transport::Http` (reqwest, REST `/api/...`) и `Transport::Grpc` (tonic, код из `proto/blog.proto`). Хранит JWT после `register` / `login`. |
| **blog-cli** | Консольная утилита поверх `blog-client`: подкоманды register, login, CRUD постов, list; токен сохраняется в `.blog_token` в текущей директории. |
| **blog-wasm** | Фронт в браузере: `wasm-bindgen`, HTTP через `gloo-net`, JWT в `localStorage` (ключ `blog_token`). Сборка даёт JS-glue в `blog-wasm/pkg/`. |

Связь по данным: **blog-server** — источник истины в БД. **blog-client** не зависит от сервера как от крейта, только совместим по контрактам API.

В **одном workspace** все четыре крейта лежат «рядом» в дереве каталогов — это не рисунок зависимостей. По **Cargo** бинарник **blog-cli** объявляет зависимость на библиотеку **blog-client** и вызывает её из своего `main`. **blog-wasm** к **`blog-client` не подключается**: в браузере HTTP идёт через `gloo-net` напрямую к REST сервера.

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

Поднимется Postgres с пользователем `blog_user`, паролем `blog_pass`, БД `blog_db`, порт **5432**. Строка подключения может совпадать с `.env.example`.

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

## Запуск компонентов

### Сервер (`blog-server`)

```bash
cargo run --bin blog-server
```

Ожидаемо:

- HTTP: `http://127.0.0.1:8080` (или `HTTP_PORT`).
- gRPC: порт `50051` (или `GRPC_PORT`).
- Проверка живости: `GET /health`.

Остановка: `Ctrl+C`.

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
cargo run -p blog-cli -- login --username alice --password secret123

# Посты (нужен предыдущий login/register в этой же директории)
cargo run -p blog-cli -- create --title "Привет" --content "Текст поста"
cargo run -p blog-cli -- list --limit 20 --offset 0
cargo run -p blog-cli -- get --id 1
cargo run -p blog-cli -- update --id 1 --title "Новый заголовок" --content "Новый текст"
cargo run -p blog-cli -- delete --id 1

# Тот же сценарий по gRPC
cargo run -p blog-cli -- --grpc register --username bob --email bob@example.com --password secret456
cargo run -p blog-cli -- --grpc login --username bob --password secret456
cargo run -p blog-cli -- --grpc create --title "gRPC пост" --content "Содержание"
```

### WASM-фронт (`blog-wasm`)

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

2. Поднимите статический сервер в **корне проекта** (рядом с `index.html`):

   ```bash
   python3 -m http.server 8000
   ```

3. Откройте в браузере страницу с хоста и порта сервера (например `http://localhost:8000`). В форме укажите базовый URL API, например `http://127.0.0.1:8080`.

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

# Ответ содержит token — подставьте в переменную:
export TOKEN="<jwt из ответа>"

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

Полный цикл: `register` → `create` → `list` → `get` → `update` → `delete` (команды выше).

### Браузер

1. Запущены Postgres, `blog-server`, собран `blog-wasm/pkg`, открыт `index.html` через локальный HTTP-сервер.
2. Указать базовый URL API.
3. Зарегистрироваться или войти — токен сохранится в `localStorage`.
4. Создать пост; для своих постов доступны изменение и удаление.

---

## Полезное

- Токен CLI: файл **`.blog_token`** в рабочей директории (не коммитится).
- Ошибки и запросы сервера логируются через **tracing** (уровень можно задать переменной окружения, например `RUST_LOG=info`).
- CORS на HTTP настроен пермиссивно для локальной разработки фронта.

## Лицензия

Как в **Cargo.toml** workspace: `MIT OR Apache-2.0`.
