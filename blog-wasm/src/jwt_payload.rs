//! Извлечение `user_id` / `username` из середины JWT (без проверки подписи — только для UI).

use base64::{engine::general_purpose::URL_SAFE, Engine as _};

/// Читает payload JWT и возвращает пары полей как на сервере блога.
pub fn claims_from_jwt_payload_unverified(token: &str) -> Option<(i64, String)> {
    let payload_b64 = token.split('.').nth(1)?;
    let mut padded = payload_b64.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    let bytes = URL_SAFE.decode(padded.as_bytes()).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let user_id = v.get("user_id").and_then(|x| x.as_i64())?;
    let username = v.get("username").and_then(|x| x.as_str())?;
    Some((user_id, username.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Минимальный payload `{"user_id":7,"username":"n"}` в URL-safe base64 без padding.
    #[test]
    fn decodes_well_formed_middle_segment() {
        let payload_json = br#"{"user_id":7,"username":"sam"}"#;
        let b64 = URL_SAFE
            .encode(payload_json)
            .trim_end_matches('=')
            .to_string();
        let token = format!("hdr.{b64}.sig");
        let (id, name) = claims_from_jwt_payload_unverified(&token).expect("parse");
        assert_eq!(id, 7);
        assert_eq!(name, "sam");
    }

    #[test]
    fn rejects_single_segment() {
        assert!(claims_from_jwt_payload_unverified("nosegments").is_none());
    }
}
