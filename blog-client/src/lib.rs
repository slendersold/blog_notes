//! Библиотека клиента блога: выбор между HTTP (`reqwest`) и gRPC (`tonic`), единые типы и ошибки.
//!
//! Совпадает с эндпоинтами `blog-server` (REST префикс `/api`) и сообщениями `proto/blog.proto`.

#![forbid(unsafe_code)]
#![allow(clippy::result_large_err)] // `tonic::Status` в `BlogClientError::Grpc` раздувает Err

pub mod blog_pb {
    tonic::include_proto!("blog");
}

mod error;
mod grpc_client;
mod http_client;

pub use error::BlogClientError;

use blog_pb::blog_service_client::BlogServiceClient;

/// Как связаться с сервером: строка задаёт базовый URL HTTP (`http://host:8080`) или tonic endpoint (`http://host:50051`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transport {
    Http(String),
    Grpc(String),
}

/// Ответ успешной регистрации или входа.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

/// Пользователь в ответах API (открытые поля без хеша пароля и соли).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Пост как сущность приложения клиента (отдельно от proto-сообщения `Post`).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    /// Имя автора для интерфейса (не email).
    pub author_username: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Результат списка с учётом пагинации (как поля REST ответа `GET /api/posts`).
#[derive(Debug, Clone)]
pub struct ListPostsOutcome {
    pub posts: Vec<Post>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Удобный алиас `Result`.
pub type Result<T> = std::result::Result<T, BlogClientError>;

fn http_reject_mapped(status: reqwest::StatusCode, message: impl Into<String>) -> BlogClientError {
    let message = message.into();
    match status {
        reqwest::StatusCode::NOT_FOUND => BlogClientError::NotFound,
        reqwest::StatusCode::UNAUTHORIZED => BlogClientError::Unauthorized,
        reqwest::StatusCode::FORBIDDEN
        | reqwest::StatusCode::BAD_REQUEST
        | reqwest::StatusCode::CONFLICT => BlogClientError::InvalidRequest { message },
        other => BlogClientError::HttpReject {
            status: other,
            message,
        },
    }
}

fn map_http_chain(e: BlogClientError) -> BlogClientError {
    match &e {
        BlogClientError::HttpReject { status, message } => {
            http_reject_mapped(*status, message.clone())
        }
        _ => e,
    }
}

/// Фасад вызова бэкенда: держит выбранный транспорт и опционально JWT для мутаций постов.
#[derive(Clone)]
pub struct BlogClient {
    pub transport: Transport,
    pub http_client: Option<reqwest::Client>,
    /// Клонированное соединение gRPC между вызовами.
    pub grpc_client: Option<BlogServiceClient<tonic::transport::Channel>>,
    pub token: Option<String>,
}

impl BlogClient {
    /// Создаёт клиента и выполняет сетевую инициализацию транспортного слоя (HTTP-пул или gRPC `Endpoint::connect`).
    pub async fn new(transport: Transport) -> Result<Self> {
        Ok(match transport.clone() {
            Transport::Http(_) => Self {
                transport,
                http_client: Some(
                    reqwest::Client::builder()
                        .build()
                        .map_err(|e: reqwest::Error| BlogClientError::from(e))?,
                ),
                grpc_client: None,
                token: None,
            },
            Transport::Grpc(endpoint) => {
                let endpoint = tonic::transport::Endpoint::from_shared(endpoint)
                    .map_err(|e: tonic::transport::Error| BlogClientError::from(e))?;
                let ch = endpoint
                    .connect()
                    .await
                    .map_err(|e: tonic::transport::Error| BlogClientError::from(e))?;
                let grpc_client = BlogServiceClient::new(ch);
                Self {
                    transport,
                    http_client: None,
                    grpc_client: Some(grpc_client),
                    token: None,
                }
            }
        })
    }

    fn http(&self) -> Result<&reqwest::Client> {
        self.http_client
            .as_ref()
            .ok_or_else(|| BlogClientError::InvalidRequest {
                message: "HTTP client not initialized".into(),
            })
    }

    fn grpc_mut(&mut self) -> Result<&mut BlogServiceClient<tonic::transport::Channel>> {
        self.grpc_client
            .as_mut()
            .ok_or_else(|| BlogClientError::InvalidRequest {
                message: "gRPC client not initialized".into(),
            })
    }

    /// Устанавливает JWT после `register`, `login` или внешней выдачи токена.
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    /// Текущий токен, если сохранён.
    pub fn get_token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse> {
        let out = match &self.transport {
            Transport::Http(base) => {
                http_client::register(self.http()?, base, username, email, password)
                    .await
                    .map_err(map_http_chain)?
            }
            Transport::Grpc(_) => {
                grpc_client::register(self.grpc_mut()?, username, email, password).await?
            }
        };
        self.token = Some(out.token.clone());
        Ok(out)
    }

    pub async fn login(&mut self, email: &str, password: &str) -> Result<AuthResponse> {
        let out = match &self.transport {
            Transport::Http(base) => http_client::login(self.http()?, base, email, password)
                .await
                .map_err(map_http_chain)?,
            Transport::Grpc(_) => grpc_client::login(self.grpc_mut()?, email, password).await?,
        };
        self.token = Some(out.token.clone());
        Ok(out)
    }

    pub async fn create_post(&mut self, title: &str, content: &str) -> Result<Post> {
        let tok = Self::require_token_owned(&self.token)?;
        match &self.transport {
            Transport::Http(base) => {
                http_client::create_post(self.http()?, base, &tok, title, content)
                    .await
                    .map_err(map_http_chain)
            }
            Transport::Grpc(_) => {
                grpc_client::create_post(self.grpc_mut()?, &tok, title, content).await
            }
        }
    }

    pub async fn get_post(&mut self, id: i64) -> Result<Post> {
        match &self.transport {
            Transport::Http(base) => http_client::get_post(self.http()?, base, id)
                .await
                .map_err(map_http_chain),
            Transport::Grpc(_) => grpc_client::get_post(self.grpc_mut()?, id).await,
        }
    }

    pub async fn update_post(&mut self, id: i64, title: &str, content: &str) -> Result<Post> {
        let tok = Self::require_token_owned(&self.token)?;
        match &self.transport {
            Transport::Http(base) => {
                http_client::update_post(self.http()?, base, &tok, id, title, content)
                    .await
                    .map_err(map_http_chain)
            }
            Transport::Grpc(_) => {
                grpc_client::update_post(self.grpc_mut()?, &tok, id, title, content).await
            }
        }
    }

    pub async fn delete_post(&mut self, id: i64) -> Result<()> {
        let tok = Self::require_token_owned(&self.token)?;
        match &self.transport {
            Transport::Http(base) => http_client::delete_post(self.http()?, base, &tok, id)
                .await
                .map_err(map_http_chain),
            Transport::Grpc(_) => grpc_client::delete_post(self.grpc_mut()?, &tok, id).await,
        }
    }

    pub async fn list_posts(&mut self, limit: i64, offset: i64) -> Result<ListPostsOutcome> {
        match &self.transport {
            Transport::Http(base) => http_client::list_posts(self.http()?, base, limit, offset)
                .await
                .map_err(map_http_chain),
            Transport::Grpc(_) => grpc_client::list_posts(self.grpc_mut()?, limit, offset).await,
        }
    }

    fn require_token_owned(token: &Option<String>) -> Result<String> {
        let t = token
            .as_ref()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .ok_or(BlogClientError::Unauthorized)?;
        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use super::BlogClientError;
    use tonic::Status;

    #[test]
    fn grpc_not_found_maps_to_not_found() {
        let e = BlogClientError::from_grpc_status(Status::not_found("x"));
        assert!(matches!(e, BlogClientError::NotFound));
    }

    #[test]
    fn grpc_unauthenticated_maps_unauthorized() {
        let e = BlogClientError::from_grpc_status(Status::unauthenticated("x"));
        assert!(matches!(e, BlogClientError::Unauthorized));
    }

    #[test]
    fn grpc_invalid_argument_maps_invalid_request() {
        let e = BlogClientError::from_grpc_status(Status::invalid_argument("bad"));
        assert!(matches!(
            e,
            BlogClientError::InvalidRequest { message } if message == "bad"
        ));
    }

    #[test]
    fn transport_clone_eq() {
        let a = super::Transport::Http("http://h".into());
        assert_eq!(a.clone(), a);
    }
}
