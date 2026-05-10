//! Выбор строки базового HTTP API и сборка заголовков/запросов для `BlogClient`.

use reqwest::{Client, Url};

use crate::error::BlogClientError;
use crate::{AuthResponse, ListPostsOutcome, Post};

fn join_base(base: &str, path: &str) -> Result<Url, BlogClientError> {
    let b = base.trim_end_matches('/');
    Url::parse(b)
        .map_err(|_| BlogClientError::InvalidHttpResponse {
            message: format!("invalid base URL {base:?}"),
        })?
        .join(path)
        .map_err(|_| BlogClientError::InvalidHttpResponse {
            message: format!("cannot join path {path:?}"),
        })
}

async fn read_error_body(resp: reqwest::Response) -> String {
    resp.text()
        .await
        .unwrap_or_else(|_| "(failed to read body)".to_string())
}

/// Пытается вытащить поле `error` из JSON тела; иначе возвращает сырой текст (усечённо).
fn parse_error_json(body: &str) -> String {
    #[derive(serde::Deserialize)]
    struct ErrBody {
        error: String,
    }
    serde_json::from_str::<ErrBody>(body)
        .map(|e| e.error)
        .unwrap_or_else(|_| {
            let t = body.trim();
            if t.len() > 256 {
                format!("{}…", &t[..256])
            } else {
                t.to_string()
            }
        })
}

pub async fn register(
    client: &Client,
    base: &str,
    username: &str,
    email: &str,
    password: &str,
) -> Result<AuthResponse, BlogClientError> {
    let url = join_base(base, "/api/auth/register")?;
    let resp = client
        .post(url)
        .json(&serde_json::json!({
            "username": username,
            "email": email,
            "password": password,
        }))
        .send()
        .await?;

    let status = resp.status();
    let body = read_error_body(resp).await;
    if status.is_success() {
        return serde_json::from_str(&body).map_err(|e| BlogClientError::InvalidHttpResponse {
            message: format!("register JSON: {e}; body: {body}"),
        });
    }
    Err(BlogClientError::HttpReject {
        status,
        message: parse_error_json(&body),
    })
}

pub async fn login(
    client: &Client,
    base: &str,
    email: &str,
    password: &str,
) -> Result<AuthResponse, BlogClientError> {
    let url = join_base(base, "/api/auth/login")?;
    let resp = client
        .post(url)
        .json(&serde_json::json!({
            "email": email,
            "password": password,
        }))
        .send()
        .await?;

    let status = resp.status();
    let body = read_error_body(resp).await;
    if status.is_success() {
        return serde_json::from_str(&body).map_err(|e| BlogClientError::InvalidHttpResponse {
            message: format!("login JSON: {e}; body: {body}"),
        });
    }
    Err(BlogClientError::HttpReject {
        status,
        message: parse_error_json(&body),
    })
}

pub async fn create_post(
    client: &Client,
    base: &str,
    token: &str,
    title: &str,
    content: &str,
) -> Result<Post, BlogClientError> {
    let url = join_base(base, "/api/posts")?;
    let resp = client
        .post(url)
        .bearer_auth(token)
        .json(&serde_json::json!({ "title": title, "content": content }))
        .send()
        .await?;

    let status = resp.status();
    let body = read_error_body(resp).await;
    if status.is_success() {
        return serde_json::from_str(&body).map_err(|e| BlogClientError::InvalidHttpResponse {
            message: format!("create_post JSON: {e}; body: {body}"),
        });
    }
    Err(BlogClientError::HttpReject {
        status,
        message: parse_error_json(&body),
    })
}

pub async fn get_post(client: &Client, base: &str, id: i64) -> Result<Post, BlogClientError> {
    let url = join_base(base, &format!("/api/posts/{id}"))?;
    let resp = client.get(url).send().await?;

    let status = resp.status();
    let body = read_error_body(resp).await;
    if status.is_success() {
        return serde_json::from_str(&body).map_err(|e| BlogClientError::InvalidHttpResponse {
            message: format!("get_post JSON: {e}; body: {body}"),
        });
    }
    Err(BlogClientError::HttpReject {
        status,
        message: parse_error_json(&body),
    })
}

pub async fn update_post(
    client: &Client,
    base: &str,
    token: &str,
    id: i64,
    title: &str,
    content: &str,
) -> Result<Post, BlogClientError> {
    let url = join_base(base, &format!("/api/posts/{id}"))?;
    let resp = client
        .put(url)
        .bearer_auth(token)
        .json(&serde_json::json!({ "title": title, "content": content }))
        .send()
        .await?;

    let status = resp.status();
    let body = read_error_body(resp).await;
    if status.is_success() {
        return serde_json::from_str(&body).map_err(|e| BlogClientError::InvalidHttpResponse {
            message: format!("update_post JSON: {e}; body: {body}"),
        });
    }
    Err(BlogClientError::HttpReject {
        status,
        message: parse_error_json(&body),
    })
}

pub async fn delete_post(
    client: &Client,
    base: &str,
    token: &str,
    id: i64,
) -> Result<(), BlogClientError> {
    let url = join_base(base, &format!("/api/posts/{id}"))?;
    let resp = client.delete(url).bearer_auth(token).send().await?;

    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = read_error_body(resp).await;
    Err(BlogClientError::HttpReject {
        status,
        message: parse_error_json(&body),
    })
}

#[derive(serde::Deserialize)]
struct ListPostsBody {
    posts: Vec<Post>,
    total: i64,
    limit: i64,
    offset: i64,
}

pub async fn list_posts(
    client: &Client,
    base: &str,
    limit: i64,
    offset: i64,
) -> Result<ListPostsOutcome, BlogClientError> {
    let url = join_base(base, "/api/posts")?;
    let resp = client
        .get(url)
        .query(&[("limit", limit.to_string()), ("offset", offset.to_string())])
        .send()
        .await?;

    let status = resp.status();
    let body = read_error_body(resp).await;
    if status.is_success() {
        let parsed: ListPostsBody =
            serde_json::from_str(&body).map_err(|e| BlogClientError::InvalidHttpResponse {
                message: format!("list_posts JSON: {e}; body: {body}"),
            })?;
        return Ok(ListPostsOutcome {
            posts: parsed.posts,
            total: parsed.total,
            limit: parsed.limit,
            offset: parsed.offset,
        });
    }
    Err(BlogClientError::HttpReject {
        status,
        message: parse_error_json(&body),
    })
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn join_base_strips_slash_and_appends_api_path() {
        let u = join_base("http://127.0.0.1:8080/", "/api/posts").unwrap();
        assert_eq!(u.as_str(), "http://127.0.0.1:8080/api/posts");
    }

    #[test]
    fn parse_error_json_extracts_field() {
        assert_eq!(parse_error_json(r#"{"error":"boom"}"#), "boom");
    }

    #[test]
    fn parse_error_json_fallback_non_json() {
        assert_eq!(parse_error_json("plain"), "plain");
    }
}
