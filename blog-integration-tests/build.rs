//! Проверка наличия `blog-cli` / `blog-server`: `CARGO_TARGET_DIR`, затем `tmp/target`, затем `target/`
//! (в workspace задано `target-dir = tmp/target` — см. `.cargo/config.toml`).
//! Вложенный `cargo build` сюда **не добавлять**: родитель держит lock на `target`, дочерний cargo может зависнуть минутами.
//!
//! Перед интеграционным тестом: `cargo build -p blog-cli -p blog-server`

use std::path::{Path, PathBuf};

fn exe_path(target: &Path, profile: &str, stem: &str) -> PathBuf {
    target
        .join(profile)
        .join(format!("{stem}{}", std::env::consts::EXE_SUFFIX))
}

/// Каталог цели: env, затем `tmp/target` (как `.cargo/config.toml` в этом workspace), затем классический `target/`.
fn target_roots(workspace: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(raw) = std::env::var("CARGO_TARGET_DIR") {
        let p = PathBuf::from(raw);
        roots.push(if p.is_absolute() {
            p
        } else {
            workspace.join(p)
        });
    }
    roots.push(workspace.join("tmp/target"));
    roots.push(workspace.join("target"));
    roots.into_iter().fold(Vec::<PathBuf>::new(), |mut acc, p| {
        if !acc.iter().any(|x| x == &p) {
            acc.push(p);
        }
        acc
    })
}

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace = manifest_dir
        .parent()
        .expect("blog-integration-tests лежит внутри каталога workspace");

    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("blog-cli/src/main.rs").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("blog-server/src/main.rs").display()
    );

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let roots = target_roots(&workspace);

    for root in &roots {
        let cli = exe_path(root, &profile, "blog-cli");
        let srv = exe_path(root, &profile, "blog-server");
        if cli.is_file() && srv.is_file() {
            return;
        }
    }

    let dirs = roots
        .iter()
        .map(|r| format!("  - {}", r.display()))
        .collect::<Vec<_>>()
        .join("\n");

    panic!(
        "Нет blog-cli и blog-server в target/{profile}/ (искали по путям ниже).\n{dirs}\n\n\
         Из корня workspace:\n  cargo build -p blog-cli -p blog-server\n"
    );
}
