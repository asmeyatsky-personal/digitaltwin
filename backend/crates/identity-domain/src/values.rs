use crate::errors::DomainError;
use kernel::PiiString;
use serde::Serialize;

/// Email value object. Invariants enforced in `parse` (§3.4). Stored as
/// `PiiString` so Debug/serde output never leaks the address.
#[derive(Clone, Serialize)]
pub struct Email(PiiString);

impl Email {
    /// # Errors
    /// Returns `DomainError::InvalidEmail` if the input fails basic RFC-5321
    /// shape checks. We deliberately use a conservative rule — a single `@`
    /// with non-empty local and domain parts — rather than a permissive regex;
    /// deep validation is deferred to the mail provider.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        let (local, domain) = trimmed.split_once('@').ok_or(DomainError::InvalidEmail)?;
        if local.is_empty() || domain.is_empty() || !domain.contains('.') {
            return Err(DomainError::InvalidEmail);
        }
        Ok(Self(PiiString::new(trimmed.to_lowercase())))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.expose()
    }
}

impl std::fmt::Debug for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Email").field(&self.0).finish()
    }
}

/// Opaque hash output. The algorithm (Argon2id) lives in infrastructure —
/// domain only knows "this is a string that verifies against a plaintext
/// via the `PasswordHasher` port".
#[derive(Clone, Serialize)]
pub struct PasswordHash(String);

impl PasswordHash {
    #[must_use]
    pub fn from_raw(raw: String) -> Self {
        Self(raw)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PasswordHash(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_parse_accepts_canonical() {
        let e = Email::parse("Alice@EXAMPLE.com").expect("valid");
        assert_eq!(e.as_str(), "alice@example.com");
    }

    #[test]
    fn email_parse_rejects_missing_at() {
        assert_eq!(
            Email::parse("no-at-sign").unwrap_err(),
            DomainError::InvalidEmail
        );
    }

    #[test]
    fn email_parse_rejects_empty_local() {
        assert_eq!(
            Email::parse("@example.com").unwrap_err(),
            DomainError::InvalidEmail
        );
    }

    #[test]
    fn email_parse_rejects_no_dot_in_domain() {
        assert_eq!(Email::parse("a@b").unwrap_err(), DomainError::InvalidEmail);
    }

    #[test]
    fn email_debug_redacts_address() {
        let e = Email::parse("alice@example.com").expect("valid");
        let rendered = format!("{e:?}");
        assert!(rendered.contains("<redacted>"), "got: {rendered}");
        assert!(!rendered.contains("alice"), "got: {rendered}");
    }

    #[test]
    fn password_hash_debug_redacts() {
        let h = PasswordHash::from_raw("argon2id$m=19456,t=2,p=1$...".into());
        assert_eq!(format!("{h:?}"), "PasswordHash(<redacted>)");
        assert!(h.as_str().starts_with("argon2id$"));
    }
}
