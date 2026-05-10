//! Лёгкий smoke без Postgres / Docker.
//!
//! Разрешение пути к бинарнику — тот же алгоритм, что `cargo_bin_hyphen` в
//! `blog-integration-tests/tests/cli_five_clients_integration.rs` (в т.ч. `tmp/target` из `.cargo/config.toml`).

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("blog-cli is a workspace member")
        .to_path_buf()
}

/// Как в `blog-integration-tests`: workspace кладёт артефакты в `tmp/target` (см. `.cargo/config.toml`).
fn blog_cli_exe() -> PathBuf {
    let key = "CARGO_BIN_EXE_blog_cli";
    if let Some(p) = std::env::var_os(key).map(PathBuf::from) {
        if p.is_file() {
            return p;
        }
    }
    let ws = workspace_root();
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let stem = format!("blog-cli{}", std::env::consts::EXE_SUFFIX);

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
        "blog-cli executable not found under target dirs (see .cargo/config.toml). \
         Run `cargo build -p blog-cli` or set {key}."
    );
}

#[test]
fn blog_cli_help_binary_smoke() {
    let exe = blog_cli_exe();
    let out = std::process::Command::new(exe)
        .args(["--help"])
        .output()
        .expect("blog-cli --help");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("register") && stdout.contains("login"),
        "{stdout}"
    );
}
