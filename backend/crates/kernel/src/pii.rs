use serde::Serialize;
use std::fmt;

/// Wrapper that redacts a sensitive value from Debug/Display/serde output.
/// Domain models store raw values (email, token) behind this type so that
/// accidental logging or JSON emission cannot leak PII — the "zero PII" part
/// of §6 enforced at the type level.
#[derive(Clone)]
pub struct PiiString(String);

impl PiiString {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Debug for PiiString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PiiString(<redacted>)")
    }
}

impl fmt::Display for PiiString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl Serialize for PiiString {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("<redacted>")
    }
}
