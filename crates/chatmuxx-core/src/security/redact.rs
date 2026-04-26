const SECRET_KEYS: &[&str] = &[
    "authorization",
    "bot_token",
    "context_token",
    "token",
    "typing_ticket",
    "upload_url",
];

pub fn mask_secret(value: &str) -> String {
    let char_count = value.chars().count();
    if char_count <= 8 {
        return "***".to_owned();
    }

    let prefix: String = value.chars().take(4).collect();
    let suffix: String = value
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    format!("{prefix}…{suffix}")
}

pub fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', '_'], "");
    SECRET_KEYS.iter().any(|secret| {
        let secret = secret.replace(['-', '_'], "");
        normalized.contains(&secret)
    })
}

pub fn redact_key_value(key: &str, value: &str) -> String {
    if is_sensitive_key(key) {
        mask_secret(value)
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_secret_hides_short_values_completely() {
        assert_eq!(mask_secret("secret"), "***");
    }

    #[test]
    fn mask_secret_keeps_small_context_for_long_values() {
        assert_eq!(mask_secret("abcdefghijklmnop"), "abcd…mnop");
    }

    #[test]
    fn sensitive_key_detection_handles_common_spellings() {
        assert!(is_sensitive_key("Authorization"));
        assert!(is_sensitive_key("bot_token"));
        assert!(is_sensitive_key("context-token"));
        assert!(is_sensitive_key("typing_ticket"));
        assert!(is_sensitive_key("upload_url"));
    }

    #[test]
    fn redact_key_value_only_masks_sensitive_keys() {
        assert_eq!(
            redact_key_value("bot_token", "abcdefghijklmnop"),
            "abcd…mnop"
        );
        assert_eq!(
            redact_key_value("workspace", "/tmp/project"),
            "/tmp/project"
        );
    }
}
