//! Слой представления: входящие запросы Actix/Tonic превращаются в вызовы сервисов.

pub mod grpc_service;
pub mod http_error;
pub mod http_handlers;
pub mod middleware;
