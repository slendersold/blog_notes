//! Регистрация и вход: хеширование пароля Argon2, выдача JWT, обёртка над `UserRepository`.

use std::sync::Arc;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use password_hash::rand_core::OsRng;

use crate::{
    data::user_repository::UserRepository,
    domain::{
        error::DomainError,
        user::{LoginRequest, RegisterRequest, User},
        validation,
    },
    infrastructure::jwt::JwtService,
};

/// Сервис аутентификации; хранит репозиторий пользователей и общий [`JwtService`] в `Arc`.
#[derive(Clone)]
pub struct AuthService {
    users: UserRepository,
    jwt: Arc<JwtService>,
}

impl AuthService {
    /// Собирает сервис из зависимостей, созданных в `main`.
    pub fn new(users: UserRepository, jwt: Arc<JwtService>) -> Self {
        Self { users, jwt }
    }

    /// Создаёт пользователя с уникальными username/email и возвращает готовый Bearer-токен и запись.
    ///
    /// Пароль никогда не хранится в открытом виде: в БД пишется соль (`password_salt`) и полная PHC-строка Argon2 (`password_hash`).
    pub async fn register(&self, req: RegisterRequest) -> Result<(String, User), DomainError> {
        let username = req.username.trim().to_string();
        let email = req.email.trim().to_string();
        validation::register_input(&username, &email, &req.password)?;
        let (password_hash, password_salt) = Self::hash_password(&req.password)?;
        let user = self
            .users
            .insert(&username, &email, &password_salt, &password_hash)
            .await?;
        let token = self
            .jwt
            .generate_token(user.id, &user.username)
            .map_err(|e| DomainError::DataBaseInternal(e.to_string()))?;
        Ok((token, user))
    }

    /// Проверяет пароль и при успехе возвращает токен и пользователя.
    ///
    /// Для единообразия с ТЗ «InvalidCredentials» скрывает отсутствие пользователя и неверный пароль.
    pub async fn login(&self, req: LoginRequest) -> Result<(String, User), DomainError> {
        let username = req.username.trim();
        validation::login_input(username, &req.password)?;
        let user = match self.users.get_by_username(username).await {
            Ok(u) => u,
            Err(DomainError::UserNotFound) => return Err(DomainError::InvalidCredentials),
            Err(e) => return Err(e),
        };

        let parsed = PasswordHash::new(&user.password_hash)
            .map_err(|_| DomainError::DataBaseInternal("bad password hash in database".into()))?;

        // Учётки до колонки `password_salt`: пустая строка; для новых строк соль в БД должна совпадать с PHC.
        if !user.password_salt.is_empty() {
            let expected = user.password_salt.as_str();
            match parsed.salt.as_ref() {
                Some(s) if s.as_str() == expected => {}
                _ => {
                    return Err(DomainError::DataBaseInternal(
                        "password_salt column out of sync with password_hash".into(),
                    ));
                }
            }
        }

        Argon2::default()
            .verify_password(req.password.as_bytes(), &parsed)
            .map_err(|_| DomainError::InvalidCredentials)?;

        let token = self
            .jwt
            .generate_token(user.id, &user.username)
            .map_err(|e| DomainError::DataBaseInternal(e.to_string()))?;
        Ok((token, user))
    }

    /// Проверяет строку Bearer-токена и возвращает claims для защищённых обработчиков.
    pub fn verify_bearer(
        &self,
        token: &str,
    ) -> Result<crate::infrastructure::jwt::Claims, DomainError> {
        self.jwt
            .verify_token(token)
            .map_err(|_| DomainError::Unauthorized)
    }

    /// Генерирует случайную соль и возвращает пару (PHC-хеш для колонки `password_hash`, строка соли для колонки `password_salt`).
    fn hash_password(password: &str) -> Result<(String, String), DomainError> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| DomainError::DataBaseInternal(e.to_string()))?;
        let password_salt = salt.as_str().to_string();
        Ok((password_hash, password_salt))
    }
}
