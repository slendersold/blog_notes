//! Выпуск и проверка JWT на основе HMAC секрета; claims несёт `user_id` и имя пользователя до истечения срока.

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// То, что кладётся в полезную нагрузку токена и при успешном decode попадает в хендлеры.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i64,
    pub username: String,
    /// Время истечения в секундах с UNIX-эпохи (`exp` для библиотеки `jsonwebtoken`).
    pub exp: usize,
}

/// Оборачивает закодированные ключи подписи/проверки; клонируется в Arc между слоями.
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    /// Собирает сервис; при сликом коротком секрете возвращает ошибку, чтобы конфигурация была явной.
    pub fn try_new(secret: &str) -> anyhow::Result<Self> {
        if secret.len() < 32 {
            anyhow::bail!(
                "JWT_SECRET must be at least 32 ASCII characters long (minimum required by coursework)"
            );
        }
        Ok(Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        })
    }

    /// Формирует строковый Bearer-токен со сроком жизни 24 часа.
    pub fn generate_token(&self, user_id: i64, username: &str) -> anyhow::Result<String> {
        let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
        let claims = Claims {
            user_id,
            username: username.to_string(),
            exp,
        };
        encode(&Header::default(), &claims, &self.encoding_key).map_err(anyhow::Error::from)
    }

    /// Проверяет подпись и срок действия и возвращает извлечённые claims или ошибку decode.
    pub fn verify_token(&self, token: &str) -> anyhow::Result<Claims> {
        decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|d| d.claims)
            .map_err(anyhow::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn try_new_rejects_short_secret() {
        assert!(JwtService::try_new("short").is_err());
    }

    #[test]
    fn generate_and_verify_roundtrip() {
        let j = JwtService::try_new(SECRET).unwrap();
        let t = j.generate_token(42, "bob").unwrap();
        let c = j.verify_token(&t).unwrap();
        assert_eq!(c.user_id, 42);
        assert_eq!(c.username, "bob");
    }

    #[test]
    fn wrong_secret_fails_verify() {
        let a = JwtService::try_new(SECRET).unwrap();
        let t = a.generate_token(1, "u").unwrap();
        let b = JwtService::try_new("fedcba9876543210fedcba9876543210").unwrap();
        assert!(b.verify_token(&t).is_err());
    }
}
