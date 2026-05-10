//! Точка входа CLI: `clap`, `blog-client`, токен в `.blog_token`, опционально `.env` из рабочей директории.

use std::fs;
use std::path::Path;

use anyhow::Context;
use clap::{Parser, Subcommand};

use blog_client::{BlogClient, ListPostsOutcome, Post, Transport, User};

const TOKEN_FILE: &str = ".blog_token";
const DEFAULT_HTTP: &str = "http://127.0.0.1:8080";
const DEFAULT_GRPC: &str = "http://127.0.0.1:50051";

/// Утилита проверки бэкенда блога: регистрация, JWT, CRUD постов по HTTP или gRPC.
#[derive(Parser, Debug)]
#[command(name = "blog-cli", version, about)]
struct Cli {
    /// Использовать gRPC (по умолчанию — HTTP).
    #[arg(long, global = true)]
    grpc: bool,
    /// Адрес сервера: для HTTP — база вроде `http://127.0.0.1:8080`, для gRPC — `http://...:50051`.
    #[arg(long, global = true)]
    server: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Регистрация; сохраняет JWT в `.blog_token`.
    Register {
        #[arg(long)]
        username: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        password: String,
    },
    /// Вход по email регистрации; перезаписывает `.blog_token`.
    Login {
        #[arg(long)]
        email: String,
        #[arg(long)]
        password: String,
    },
    /// Создать пост (нужен токен из register/login).
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: String,
    },
    /// Показать пост по id.
    Get {
        #[arg(long)]
        id: i64,
    },
    /// Обновить пост (только автор; нужен токен).
    Update {
        #[arg(long)]
        id: i64,
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: String,
    },
    /// Удалить пост (только автор; нужен токен).
    Delete {
        #[arg(long)]
        id: i64,
    },
    /// Список с пагинацией.
    List {
        #[arg(long, default_value_t = 20)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let base = match (&cli.server, cli.grpc) {
        (Some(s), _) => s.clone(),
        (None, true) => DEFAULT_GRPC.to_string(),
        (None, false) => DEFAULT_HTTP.to_string(),
    };

    let transport = if cli.grpc {
        Transport::Grpc(base)
    } else {
        Transport::Http(base)
    };

    let mut client = BlogClient::new(transport)
        .await
        .map_err(|e| anyhow::Error::msg(e.to_string()))?;
    if let Some(tok) = load_token(Path::new(TOKEN_FILE))? {
        client.set_token(tok);
    }

    match run_command(&mut client, &cli.command).await {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

async fn run_command(client: &mut BlogClient, cmd: &Commands) -> anyhow::Result<()> {
    match cmd {
        Commands::Register {
            username,
            email,
            password,
        } => {
            let r = client
                .register(username, email, password)
                .await
                .map_err(|e| anyhow::Error::msg(e.to_string()))?;
            save_token(Path::new(TOKEN_FILE), &r.token)?;
            print_json(&serde_json::json!({
                "token": r.token,
                "user": user_json(&r.user),
            }))?;
        }
        Commands::Login { email, password } => {
            let r = client
                .login(email, password)
                .await
                .map_err(|e| anyhow::Error::msg(e.to_string()))?;
            save_token(Path::new(TOKEN_FILE), &r.token)?;
            print_json(&serde_json::json!({
                "token": r.token,
                "user": user_json(&r.user),
            }))?;
        }
        Commands::Create { title, content } => {
            let p = client
                .create_post(title, content)
                .await
                .map_err(|e| anyhow::Error::msg(e.to_string()))?;
            print_json(&post_json(&p))?;
        }
        Commands::Get { id } => {
            let p = client
                .get_post(*id)
                .await
                .map_err(|e| anyhow::Error::msg(e.to_string()))?;
            print_json(&post_json(&p))?;
        }
        Commands::Update { id, title, content } => {
            let p = client
                .update_post(*id, title, content)
                .await
                .map_err(|e| anyhow::Error::msg(e.to_string()))?;
            print_json(&post_json(&p))?;
        }
        Commands::Delete { id } => {
            client
                .delete_post(*id)
                .await
                .map_err(|e| anyhow::Error::msg(e.to_string()))?;
            println!("deleted post {}", id);
        }
        Commands::List { limit, offset } => {
            let r = client
                .list_posts(*limit, *offset)
                .await
                .map_err(|e| anyhow::Error::msg(e.to_string()))?;
            print_json(&list_json(&r))?;
        }
    }
    Ok(())
}

fn user_json(u: &User) -> serde_json::Value {
    serde_json::json!({
        "id": u.id,
        "username": u.username,
        "email": u.email,
        "created_at": u.created_at.to_rfc3339(),
    })
}

fn post_json(p: &Post) -> serde_json::Value {
    serde_json::json!({
        "id": p.id,
        "title": p.title,
        "content": p.content,
        "author_username": p.author_username,
        "author_id": p.author_id,
        "created_at": p.created_at.to_rfc3339(),
        "updated_at": p.updated_at.to_rfc3339(),
    })
}

fn list_json(o: &ListPostsOutcome) -> serde_json::Value {
    serde_json::json!({
        "posts": o.posts.iter().map(post_json).collect::<Vec<_>>(),
        "total": o.total,
        "limit": o.limit,
        "offset": o.offset,
    })
}

fn print_json(v: &serde_json::Value) -> anyhow::Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(v).context("serialize JSON")?
    );
    Ok(())
}

fn load_token(path: &Path) -> anyhow::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let s = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let t = s.trim().to_string();
    Ok(if t.is_empty() { None } else { Some(t) })
}

fn save_token(path: &Path, token: &str) -> anyhow::Result<()> {
    fs::write(path, token.trim().as_bytes()).with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::fs;

    #[test]
    fn clap_parses_register() {
        let cli = Cli::parse_from([
            "blog-cli",
            "register",
            "--username",
            "alice",
            "--email",
            "a@b.cd",
            "--password",
            "password12",
        ]);
        match cli.command {
            Commands::Register {
                username,
                email,
                password,
            } => {
                assert_eq!(username, "alice");
                assert_eq!(email, "a@b.cd");
                assert_eq!(password, "password12");
            }
            _ => panic!("expected register"),
        }
    }

    #[test]
    fn clap_parses_global_server() {
        let cli = Cli::parse_from([
            "blog-cli",
            "--server",
            "http://127.0.0.1:9",
            "list",
            "--limit",
            "5",
            "--offset",
            "10",
        ]);
        assert_eq!(cli.server.as_deref(), Some("http://127.0.0.1:9"));
        match cli.command {
            Commands::List { limit, offset } => {
                assert_eq!(limit, 5);
                assert_eq!(offset, 10);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn token_load_save_trim() {
        let dir = std::env::temp_dir().join(format!("blog_cli_token_{}", std::process::id()));
        let _ = fs::remove_file(&dir);
        save_token(&dir, "  tok\n").expect("save");
        assert_eq!(load_token(&dir).expect("load").as_deref(), Some("tok"));
        let _ = fs::remove_file(&dir);
    }

    #[test]
    fn post_json_contains_author_username() {
        let p = blog_client::Post {
            id: 1,
            title: "t".into(),
            content: "c".into(),
            author_id: 2,
            author_username: "bob".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let v = post_json(&p);
        assert_eq!(v["author_username"], "bob");
    }
}
