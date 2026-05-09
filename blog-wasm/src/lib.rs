//! WASM-обёртка над HTTP API сервера: `BlogApp`, localStorage `blog_token`, запросы через `gloo-net`.
//!
//! Сборка фронта: `wasm-pack build --target web` в каталоге `blog-wasm/`, либо из корня workspace: `./scripts/build-wasm-web.sh` (нужен `wasm-bindgen` из `wasm-bindgen-cli`). Артефакты — `blog-wasm/pkg/`.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use gloo_net::http::Request;
use web_sys::window;

const LS_TOKEN: &str = "blog_token";
const LS_USER: &str = "blog_user";

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AuthResponse {
    token: String,
    user: UserDto,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct UserDto {
    id: i64,
    username: String,
    email: String,
    created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PostDto {
    id: i64,
    title: String,
    content: String,
    author_id: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ListPostsBody {
    posts: Vec<PostDto>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Deserialize)]
struct ErrBody {
    error: String,
}

fn js_err(msg: impl Into<String>) -> JsValue {
    JsValue::from_str(&msg.into())
}

async fn response_json<T: serde::de::DeserializeOwned>(
    resp: gloo_net::http::Response,
) -> Result<T, JsValue> {
    let status = resp.status();
    let text = resp.text().await.map_err(|e| js_err(e.to_string()))?;
    if status >= 400 {
        if let Ok(e) = serde_json::from_str::<ErrBody>(&text) {
            return Err(js_err(e.error));
        }
        return Err(js_err(text));
    }
    serde_json::from_str(&text).map_err(|e| js_err(e.to_string()))
}

async fn response_empty(resp: gloo_net::http::Response) -> Result<(), JsValue> {
    let status = resp.status();
    if status >= 400 {
        let text = resp.text().await.unwrap_or_default();
        if let Ok(e) = serde_json::from_str::<ErrBody>(&text) {
            return Err(js_err(e.error));
        }
        return Err(js_err(text));
    }
    Ok(())
}

/// Состояние фронта: базовый URL API и сессия после входа/регистрации.
#[wasm_bindgen]
pub struct BlogApp {
    base: String,
    token: Option<String>,
    user_id: Option<i64>,
    username: Option<String>,
}

#[wasm_bindgen]
impl BlogApp {
    /// Создаёт приложение и подтягивает токен и профиль из `localStorage`, если есть.
    #[wasm_bindgen(constructor)]
    pub fn new(api_base: &str) -> BlogApp {
        let mut app = BlogApp {
            base: api_base.trim_end_matches('/').to_string(),
            token: None,
            user_id: None,
            username: None,
        };
        app.load_session_from_storage();
        app
    }

    /// Есть ли сохранённый JWT (после login/register или из storage).
    #[wasm_bindgen(js_name = isAuthenticated)]
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// Текущий пользователь `{ user_id, username }` или `null`.
    #[wasm_bindgen(js_name = session)]
    pub fn session_js(&self) -> JsValue {
        match (self.user_id, self.username.clone()) {
            (Some(id), Some(ref name)) => serde_wasm_bindgen::to_value(&serde_json::json!({
                "user_id": id,
                "username": name,
            }))
            .unwrap_or(JsValue::NULL),
            _ => JsValue::NULL,
        }
    }

    /// Регистрация; сохраняет JWT и профиль в память и в storage.
    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<JsValue, JsValue> {
        let url = format!("{}/api/auth/register", self.base);
        let body = serde_json::json!({
            "username": username,
            "email": email,
            "password": password,
        });
        let resp = Request::post(&url)
            .json(&body)
            .map_err(|e| js_err(e.to_string()))?
            .send()
            .await
            .map_err(|e| js_err(e.to_string()))?;

        let auth: AuthResponse = response_json(resp).await?;
        self.apply_auth(auth.clone());
        serde_wasm_bindgen::to_value(&auth).map_err(|e| js_err(e.to_string()))
    }

    /// Вход; сохраняет JWT и профиль.
    pub async fn login(&mut self, username: &str, password: &str) -> Result<JsValue, JsValue> {
        let url = format!("{}/api/auth/login", self.base);
        let body = serde_json::json!({
            "username": username,
            "password": password,
        });
        let resp = Request::post(&url)
            .json(&body)
            .map_err(|e| js_err(e.to_string()))?
            .send()
            .await
            .map_err(|e| js_err(e.to_string()))?;

        let auth: AuthResponse = response_json(resp).await?;
        self.apply_auth(auth.clone());
        serde_wasm_bindgen::to_value(&auth).map_err(|e| js_err(e.to_string()))
    }

    /// Очищает сессию и ключи в `localStorage`.
    pub fn logout(&mut self) {
        self.token = None;
        self.user_id = None;
        self.username = None;
        clear_auth_storage();
    }

    /// Публичный список постов (первые 20, как дефолт сервера).
    pub async fn load_posts(&self) -> Result<JsValue, JsValue> {
        let limit = 20_i64;
        let offset = 0_i64;
        let url = format!("{}/api/posts?limit={}&offset={}", self.base, limit, offset);
        let resp = Request::get(&url)
            .send()
            .await
            .map_err(|e| js_err(e.to_string()))?;
        let body: ListPostsBody = response_json(resp).await?;
        serde_wasm_bindgen::to_value(&body).map_err(|e| js_err(e.to_string()))
    }

    /// Один пост по id.
    pub async fn get_post(&self, id: i64) -> Result<JsValue, JsValue> {
        let url = format!("{}/api/posts/{id}", self.base);
        let resp = Request::get(&url)
            .send()
            .await
            .map_err(|e| js_err(e.to_string()))?;
        let post: PostDto = response_json(resp).await?;
        serde_wasm_bindgen::to_value(&post).map_err(|e| js_err(e.to_string()))
    }

    /// Создание поста (нужен Bearer).
    pub async fn create_post(&self, title: &str, content: &str) -> Result<JsValue, JsValue> {
        let tok = self
            .token
            .as_deref()
            .ok_or_else(|| js_err("not authenticated"))?;
        let url = format!("{}/api/posts", self.base);
        let body = serde_json::json!({ "title": title, "content": content });
        let resp = Request::post(&url)
            .header("Authorization", &format!("Bearer {tok}"))
            .json(&body)
            .map_err(|e| js_err(e.to_string()))?
            .send()
            .await
            .map_err(|e| js_err(e.to_string()))?;
        let post: PostDto = response_json(resp).await?;
        serde_wasm_bindgen::to_value(&post).map_err(|e| js_err(e.to_string()))
    }

    /// Обновление поста (Bearer; автор проверяется на сервере).
    pub async fn update_post(
        &self,
        id: i64,
        title: &str,
        content: &str,
    ) -> Result<JsValue, JsValue> {
        let tok = self
            .token
            .as_deref()
            .ok_or_else(|| js_err("not authenticated"))?;
        let url = format!("{}/api/posts/{id}", self.base);
        let body = serde_json::json!({ "title": title, "content": content });
        let resp = Request::put(&url)
            .header("Authorization", &format!("Bearer {tok}"))
            .json(&body)
            .map_err(|e| js_err(e.to_string()))?
            .send()
            .await
            .map_err(|e| js_err(e.to_string()))?;
        let post: PostDto = response_json(resp).await?;
        serde_wasm_bindgen::to_value(&post).map_err(|e| js_err(e.to_string()))
    }

    /// Удаление поста (Bearer).
    pub async fn delete_post(&self, id: i64) -> Result<JsValue, JsValue> {
        let tok = self
            .token
            .as_deref()
            .ok_or_else(|| js_err("not authenticated"))?;
        let url = format!("{}/api/posts/{id}", self.base);
        let resp = Request::delete(&url)
            .header("Authorization", &format!("Bearer {tok}"))
            .send()
            .await
            .map_err(|e| js_err(e.to_string()))?;
        response_empty(resp).await?;
        Ok(JsValue::UNDEFINED)
    }
}

impl BlogApp {
    fn load_session_from_storage(&mut self) {
        if let Some(t) = read_token_from_storage() {
            if !t.is_empty() {
                self.token = Some(t);
            }
        }
        if let Some(s) = read_user_json_from_storage() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let (Some(id), Some(name)) = (
                    v.get("id").and_then(|x| x.as_i64()),
                    v.get("username").and_then(|x| x.as_str()),
                ) {
                    self.user_id = Some(id);
                    self.username = Some(name.to_string());
                }
            }
        }
    }

    fn apply_auth(&mut self, auth: AuthResponse) {
        self.token = Some(auth.token.clone());
        self.user_id = Some(auth.user.id);
        self.username = Some(auth.user.username.clone());
        persist_token(&auth.token);
        let u = serde_json::json!({
            "id": auth.user.id,
            "username": auth.user.username,
        });
        persist_user_json(&serde_json::to_string(&u).unwrap_or_default());
    }
}

fn storage() -> Option<web_sys::Storage> {
    window()?.local_storage().ok()?
}

fn persist_token(token: &str) {
    if let Some(ls) = storage() {
        let _ = ls.set_item(LS_TOKEN, token);
    }
}

fn persist_user_json(json: &str) {
    if let Some(ls) = storage() {
        let _ = ls.set_item(LS_USER, json);
    }
}

fn read_token_from_storage() -> Option<String> {
    storage()?.get_item(LS_TOKEN).ok()?
}

fn read_user_json_from_storage() -> Option<String> {
    storage()?.get_item(LS_USER).ok()?
}

fn clear_auth_storage() {
    if let Some(ls) = storage() {
        let _ = ls.remove_item(LS_TOKEN);
        let _ = ls.remove_item(LS_USER);
    }
}

/// Сохранение JWT в `localStorage` под ключом `blog_token` (для ТЗ и отладки из JS).
#[wasm_bindgen]
pub fn save_token_to_storage(token: &str) {
    persist_token(token);
}

/// Чтение JWT из `localStorage`.
#[wasm_bindgen]
pub fn get_token_from_storage() -> Option<String> {
    read_token_from_storage()
}
