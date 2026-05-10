//! Ограничения длины и примитивная проверка email до походов в Postgres.
//!
//! Это сознательно упрощённый MVP: нет RFC-совместимого парсера почты, только явные ограничения для API.

use crate::domain::error::DomainError;

const USERNAME_MIN: usize = 3;
const USERNAME_MAX: usize = 64;
const PASSWORD_MIN: usize = 8;
const PASSWORD_MAX: usize = 256;
/// Заголовок поста ограничен разумным размером для индексов и UI в браузере.
pub const TITLE_MAX: usize = 512;
/// Тело поста ограничено, чтобы случайный клиент не заливал огромный TEXT одним запросом.
pub const CONTENT_MAX: usize = 65536;

/// Проверяет пользовательские поля регистрации сразу после `trim()` в сервисе.
pub fn register_input(username: &str, email: &str, password: &str) -> Result<(), DomainError> {
    let ulen = username.chars().count();
    if !(USERNAME_MIN..=USERNAME_MAX).contains(&ulen) {
        return Err(DomainError::Validation(format!(
            "username length must be between {USERNAME_MIN} and {USERNAME_MAX} Unicode scalars after trim",
        )));
    }
    if password.len() < PASSWORD_MIN || password.len() > PASSWORD_MAX {
        return Err(DomainError::Validation(format!(
            "password length must be between {PASSWORD_MIN} and {PASSWORD_MAX}",
        )));
    }
    if email.is_empty() {
        return Err(DomainError::Validation(
            "email must not be empty after trim".into(),
        ));
    }
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(DomainError::Validation(
            "email must contain exactly one @ between local and domain parts".into(),
        ));
    }
    let local = parts[0];
    let domain = parts[1];
    if local.is_empty()
        || domain.is_empty()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !domain.contains('.')
    {
        return Err(DomainError::Validation(
            "email must have non-empty local part and plausible domain segment".into(),
        ));
    }
    Ok(())
}

/// Проверка для входа по email и паролю (email — после `trim()`, как при регистрации).
pub fn login_input(email_trimmed: &str, password: &str) -> Result<(), DomainError> {
    if email_trimmed.is_empty() || password.is_empty() {
        return Err(DomainError::Validation(
            "email and password must be non-empty".into(),
        ));
    }
    validate_email_like_login(email_trimmed)?;
    if password.len() < PASSWORD_MIN || password.len() > PASSWORD_MAX {
        return Err(DomainError::Validation(format!(
            "password length must be between {PASSWORD_MIN} and {PASSWORD_MAX}",
        )));
    }
    Ok(())
}

fn validate_email_like_login(email: &str) -> Result<(), DomainError> {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(DomainError::Validation(
            "email must contain exactly one @".into(),
        ));
    }
    let local = parts[0];
    let domain = parts[1];
    if local.is_empty()
        || domain.is_empty()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !domain.contains('.')
    {
        return Err(DomainError::Validation(
            "invalid email shape for login".into(),
        ));
    }
    Ok(())
}

/// Общее ограничение для создания или обновления поста через REST/gRPC приложения.
pub fn post_payload(title: &str, content: &str) -> Result<(), DomainError> {
    if title.trim().is_empty() || content.trim().is_empty() {
        return Err(DomainError::Validation(
            "post title and content must be non-empty".into(),
        ));
    }
    if title.len() > TITLE_MAX {
        return Err(DomainError::Validation(format!(
            "title must be at most {TITLE_MAX} bytes in UTF-8",
        )));
    }
    if content.len() > CONTENT_MAX {
        return Err(DomainError::Validation(format!(
            "content must be at most {CONTENT_MAX} bytes in UTF-8",
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::error::DomainError;

    #[test]
    fn register_ok() {
        assert!(register_input("alice", "a@b.cd", "password12").is_ok());
    }

    #[test]
    fn register_username_too_short() {
        assert!(matches!(
            register_input("ab", "a@b.cd", "password12"),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn register_bad_email() {
        assert!(register_input("alice", "no-at", "password12").is_err());
        assert!(register_input("alice", "@b.cd", "password12").is_err());
        assert!(register_input("alice", "a@.cd", "password12").is_err());
    }

    #[test]
    fn login_empty_email() {
        assert!(matches!(
            login_input("", "password12"),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn post_payload_trim_empty_title() {
        assert!(post_payload("   ", "hi").is_err());
    }

    #[test]
    fn post_title_byte_limit() {
        let t = "я".repeat(TITLE_MAX / 2 + 2);
        assert!(t.len() > TITLE_MAX);
        assert!(post_payload(&t, "c").is_err());
    }
}
