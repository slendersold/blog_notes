//! Лёгкий smoke без Postgres / Docker.

use std::path::{Path, PathBuf};

fn blog_cli_exe() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_blog_cli")
        .map(PathBuf::from)
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"));
            let target = std::env::var_os("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join("target"));
            let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
            target.join(profile).join("blog-cli")
        })
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
