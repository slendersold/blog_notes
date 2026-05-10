//! Поднимает **Postgres в Docker** тем же **`docker-compose.yml`**, что и ручная разработка,
//! затем **blog-server** и пять параллельных **blog-cli** (как в `async_quotation_simulation/tests/multi_client_integration.rs`).
//!
//! Требуется **Docker Engine** и **Compose v2** (`docker compose`). Свободный порт задаётся через **`PG_HOST_PORT`**
//! (см. переменную в `docker-compose.yml`); стек изолирован от обычного `docker compose up` через **`-p`** (отдельное имя проекта).
//!
//! Перед запуском соберите бинарники: `cargo build -p blog-cli -p blog-server` (см. `build.rs` в этом крейте).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use rand::Rng as _;
use reqwest::blocking::Client as HttpBlocking;
use serde_json::Value;
use tempfile::TempDir;

const COMPOSE_FILE: &str = "docker-compose.yml";
const PG_SERVICE: &str = "postgres";

const N_CLIENTS: usize = 5;
const JWT_SECRET: &str = "0123456789abcdef0123456789abcdef";
const SHARED_PASSWORD: &str = "itPassWordIntegration42!";

struct ServerGuard(Option<std::process::Child>);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(mut c) = self.0.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// Снимает стек Postgres после теста (или при панике).
struct ComposePgGuard {
    workspace: PathBuf,
    compose_path: PathBuf,
    project: String,
}

impl Drop for ComposePgGuard {
    fn drop(&mut self) {
        let _ = docker_compose(
            &self.workspace,
            Some(&self.compose_path),
            &self.project,
            &["down", "-v"],
            None,
        );
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("blog-integration-tests внутри каталога workspace")
        .to_path_buf()
}

fn docker_compose(
    workspace: &Path,
    compose_file: Option<&Path>,
    project: &str,
    args: &[&str],
    env_pg_port: Option<u16>,
) -> std::process::ExitStatus {
    let mut cmd = Command::new("docker");
    cmd.current_dir(workspace);
    cmd.arg("compose").arg("-p").arg(project);
    if let Some(f) = compose_file {
        cmd.arg("-f").arg(f);
    }
    if let Some(p) = env_pg_port {
        cmd.env("PG_HOST_PORT", p.to_string());
    }
    for a in args {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::null())
        .status()
        .expect("spawn docker compose")
}

fn docker_compose_up_and_wait(workspace: &Path, compose_file: &Path, project: &str, pg_port: u16) {
    assert!(
        compose_file.exists(),
        "нет {} — файл должен быть в репозитории рядом с workspace",
        compose_file.display()
    );

    let up = docker_compose(
        workspace,
        Some(compose_file),
        project,
        &["up", "-d"],
        Some(pg_port),
    );
    assert!(
        up.success(),
        "`docker compose up -d` failed (нужны docker и Compose v2, см. stderr в консоли)"
    );

    let deadline = Instant::now() + Duration::from_secs(120);
    while Instant::now() < deadline {
        let rc = docker_compose(
            workspace,
            Some(compose_file),
            project,
            &[
                "exec",
                "-T",
                PG_SERVICE,
                "pg_isready",
                "-U",
                "blog_user",
                "-d",
                "blog_db",
            ],
            None,
        );
        if rc.success() {
            return;
        }
        sleep(Duration::from_millis(300));
    }
    panic!(
        "{PG_SERVICE} в Docker не прошёл pg_isready за 120 секунд — проект compose `{project}`, порт {pg_port}"
    );
}

fn cargo_bin_hyphen(name: &'static str) -> PathBuf {
    let exe_key = format!("CARGO_BIN_EXE_{}", name.replace('-', "_"));
    if let Some(p) = std::env::var_os(&exe_key).map(PathBuf::from) {
        if p.is_file() {
            return p;
        }
    }
    let ws = workspace_root();
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let stem = format!("{name}{}", std::env::consts::EXE_SUFFIX);

    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(raw) = std::env::var("CARGO_TARGET_DIR") {
        let p = PathBuf::from(raw);
        roots.push(if p.is_absolute() { p } else { ws.join(p) });
    }
    roots.push(ws.join("tmp/target"));
    roots.push(ws.join("target"));
    let roots: Vec<PathBuf> = roots.into_iter().fold(Vec::new(), |mut acc, p| {
        if !acc.iter().any(|x| x == &p) {
            acc.push(p);
        }
        acc
    });

    for root in roots {
        let candidate = root.join(&profile).join(&stem);
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "Не найден бинарник `{name}` под profile `{profile}` (tmp/target vs target см. `.cargo/config.toml`). \
         Сборка: cargo build -p blog-cli -p blog-server",
    );
}

fn pick_port() -> u16 {
    use std::net::TcpListener;
    TcpListener::bind(("127.0.0.1", 0))
        .expect("bind 0 port")
        .local_addr()
        .expect("addr")
        .port()
}

fn wait_http_health(http_base: &str, deadline: Duration) {
    let client = HttpBlocking::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("reqwest blocking");
    let url = format!("{}/health", http_base.trim_end_matches('/'));
    let started = Instant::now();
    while started.elapsed() < deadline {
        if client
            .get(&url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return;
        }
        sleep(Duration::from_millis(80));
    }
    panic!("{deadline:?}: сервер не ответил на {url}");
}

#[test]
fn five_parallel_cli_clients_one_server_postgres_http() {
    let ws = workspace_root();
    let compose_path = ws.join(COMPOSE_FILE);
    let pid = std::process::id();
    let project = format!("blog_integration_{pid}");
    let pg_port = pick_port();

    let _compose_guard = ComposePgGuard {
        workspace: ws.clone(),
        compose_path: compose_path.clone(),
        project: project.clone(),
    };

    docker_compose_up_and_wait(&ws, &compose_path, &project, pg_port);

    let database_url =
        format!("postgres://blog_user:blog_pass@127.0.0.1:{pg_port}/blog_db?sslmode=disable");

    let http_port = pick_port();
    let grpc_port = pick_port();

    let http_base = format!("http://127.0.0.1:{http_port}");

    let server_exe = cargo_bin_hyphen("blog-server");
    let child = Command::new(&server_exe)
        .env("DATABASE_URL", &database_url)
        .env("JWT_SECRET", JWT_SECRET)
        .env("HTTP_PORT", http_port.to_string())
        .env("GRPC_PORT", grpc_port.to_string())
        .env("RUST_LOG", "off")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn blog-server");
    let _server = ServerGuard(Some(child));

    wait_http_health(&http_base, Duration::from_secs(90));

    let cli_path = cargo_bin_hyphen("blog-cli");
    let test_pid = std::process::id();

    let handles: Vec<_> = (0..N_CLIENTS)
        .map(|idx| {
            let http_base = http_base.clone();
            let cli_path = cli_path.clone();
            thread::spawn(move || {
                let mut rng = rand::thread_rng();
                sleep(Duration::from_millis(rng.gen_range(0..120)));

                let dir = TempDir::new().expect("tempdir");
                let path = dir.path();

                let username = format!("user{idx}_{test_pid}");
                let email = format!("u{idx}.{test_pid}@cli-it.test");

                let reg = Command::new(&cli_path)
                    .current_dir(path)
                    .args([
                        "--server",
                        &http_base,
                        "register",
                        "--username",
                        &username,
                        "--email",
                        &email,
                        "--password",
                        SHARED_PASSWORD,
                    ])
                    .output()
                    .expect("register");
                assert!(
                    reg.status.success(),
                    "client {idx} register err: {}",
                    String::from_utf8_lossy(&reg.stderr)
                );

                let title = format!("Клиент-{idx}-post");
                let content = format!("Текст заметки от клиента {idx}");

                let cr = Command::new(&cli_path)
                    .current_dir(path)
                    .args([
                        "--server",
                        &http_base,
                        "create",
                        "--title",
                        &title,
                        "--content",
                        &content,
                    ])
                    .output()
                    .expect("create");
                assert!(
                    cr.status.success(),
                    "client {idx} create err: {}",
                    String::from_utf8_lossy(&cr.stderr)
                );

                let v: Value = serde_json::from_slice(&cr.stdout).expect("create JSON");
                let id = v["id"].as_i64().expect("post id in create response");
                let author = v["author_username"].as_str().expect("author_username");
                assert_eq!(author, username, "author_username JSON");
                (idx, id, title, email)
            })
        })
        .collect();

    let mut created: Vec<(usize, i64, String, String)> = Vec::with_capacity(N_CLIENTS);
    for h in handles {
        created.push(h.join().expect("client thread panic"));
    }
    created.sort_by_key(|x| x.0);

    let list_json = Command::new(&cli_path)
        .current_dir(workspace_root())
        .args([
            "--server", &http_base, "list", "--limit", "50", "--offset", "0",
        ])
        .output()
        .expect("public list");

    assert!(
        list_json.status.success(),
        "list stderr={}",
        String::from_utf8_lossy(&list_json.stderr),
    );

    let list_val: Value = serde_json::from_slice(&list_json.stdout).expect("list JSON");
    let posts = list_val["posts"].as_array().expect("posts array");
    assert!(
        posts.len() >= N_CLIENTS,
        "ожидали ≥{N_CLIENTS} постов, было {}",
        posts.len()
    );

    let titles: Vec<&str> = posts.iter().filter_map(|p| p["title"].as_str()).collect();
    for (_, _, ref want_title, _) in &created {
        assert!(
            titles.iter().any(|t| *t == want_title),
            "заголовок {want_title:?} отсутствует в списке: {titles:?}"
        );
    }

    for (_, id, want_title, _) in &created {
        let g = Command::new(&cli_path)
            .current_dir(workspace_root())
            .args(["--server", &http_base, "get", "--id", &id.to_string()])
            .output()
            .expect("get");
        assert!(
            g.status.success(),
            "get id={id}: {}",
            String::from_utf8_lossy(&g.stderr)
        );
        let gv: Value = serde_json::from_slice(&g.stdout).expect("get json");
        assert_eq!(gv["title"].as_str().expect("title"), want_title);
    }

    let id0 = created[0].1;
    let email0 = created[0].3.clone();
    let tail_dir = TempDir::new().expect("tail tempdir");
    let login0 = Command::new(&cli_path)
        .current_dir(tail_dir.path())
        .args([
            "--server",
            &http_base,
            "login",
            "--email",
            &email0,
            "--password",
            SHARED_PASSWORD,
        ])
        .output()
        .expect("login user0");
    assert!(
        login0.status.success(),
        "login0: {}",
        String::from_utf8_lossy(&login0.stderr)
    );

    let up = Command::new(&cli_path)
        .current_dir(tail_dir.path())
        .args([
            "--server",
            &http_base,
            "update",
            "--id",
            &id0.to_string(),
            "--title",
            "Обновлено-интеграция",
            "--content",
            "Новый текст",
        ])
        .output()
        .expect("update");
    assert!(
        up.status.success(),
        "update: {}",
        String::from_utf8_lossy(&up.stderr)
    );

    let del = Command::new(&cli_path)
        .current_dir(tail_dir.path())
        .args(["--server", &http_base, "delete", "--id", &id0.to_string()])
        .output()
        .expect("delete");
    assert!(
        del.status.success(),
        "delete: {}",
        String::from_utf8_lossy(&del.stderr)
    );

    let get_missing = Command::new(&cli_path)
        .current_dir(workspace_root())
        .args(["--server", &http_base, "get", "--id", &id0.to_string()])
        .output()
        .expect("get deleted");
    assert!(
        !get_missing.status.success(),
        "ожидали ошибку для удалённого id {id0}"
    );

    drop(_server);
    // ComposePgGuard сделает docker compose down -v
}
