//! Ошибки клиента: `snafu` для явных вариантов, `Display` и `Error` из одного места.

use snafu::Snafu;
use tonic::{Code, Status};

/// Унифицированное представление проблем запросов к серверу блога по HTTP или gRPC.
#[derive(Debug, Snafu)]
pub enum BlogClientError {
    /// Сеть, TLS, чтение тела и т.д.
    #[snafu(display("HTTP transport: {source}"))]
    Http { source: reqwest::Error },

    /// HTTP-успех по коду, тело с полем `error` (сервер блога).
    #[snafu(display("server reported: {message} ({status})"))]
    HttpReject {
        status: reqwest::StatusCode,
        message: String,
    },

    /// Не удалось разобрать ожидаемый JSON или собрать URL.
    #[snafu(display("invalid HTTP response: {message}"))]
    InvalidHttpResponse { message: String },

    /// Соединение gRPC (Endpoint::connect).
    #[snafu(display("gRPC transport: {source}"))]
    Transport { source: tonic::transport::Error },

    /// «Сырой» ответ gRPC, не сведённый к суженным вариантам.
    #[snafu(display("gRPC: {status}"))]
    Grpc { status: Status },

    #[snafu(display("resource not found"))]
    NotFound,

    #[snafu(display("unauthorized"))]
    Unauthorized,

    /// Нет токена, неверные аргументы клиента, расхождение с контрактом API.
    #[snafu(display("invalid client request: {message}"))]
    InvalidRequest { message: String },
}

impl BlogClientError {
    /// Сводит [`Status`] к узкому набору вариантов; прочие коды остаются в [`BlogClientError::Grpc`].
    pub fn from_grpc_status(s: Status) -> Self {
        match s.code() {
            Code::NotFound => Self::NotFound,
            Code::Unauthenticated => Self::Unauthorized,
            Code::InvalidArgument | Code::AlreadyExists | Code::PermissionDenied => {
                Self::InvalidRequest {
                    message: s.message().to_string(),
                }
            }
            Code::Unavailable | Code::DeadlineExceeded => Self::Grpc { status: s },
            _ => Self::Grpc { status: s },
        }
    }
}

impl From<reqwest::Error> for BlogClientError {
    fn from(source: reqwest::Error) -> Self {
        Self::Http { source }
    }
}

impl From<tonic::transport::Error> for BlogClientError {
    fn from(source: tonic::transport::Error) -> Self {
        Self::Transport { source }
    }
}
