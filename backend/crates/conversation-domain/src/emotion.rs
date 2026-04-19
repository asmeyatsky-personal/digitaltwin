use crate::errors::DomainError;
use serde::{Deserialize, Serialize};

/// Unified emotional tone taxonomy (AD-1). Every layer uses this enum; legacy
/// per-layer taxonomies (6/7/8-value variants) map onto this set at the
/// adapter boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmotionalTone {
    Neutral,
    Happy,
    Sad,
    Angry,
    Anxious,
    Surprised,
    Calm,
    Excited,
}

impl EmotionalTone {
    /// Parse the raw string an LLM adapter returns. §4 requires AI output to
    /// be validated against an explicit schema; this is the schema.
    ///
    /// # Errors
    /// `DomainError::InvalidEmotion` for any value outside the unified set.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_lowercase().as_str() {
            "neutral" => Ok(Self::Neutral),
            "happy" => Ok(Self::Happy),
            "sad" => Ok(Self::Sad),
            "angry" | "frustrated" => Ok(Self::Angry),
            "anxious" | "worried" | "concerned" | "fear" => Ok(Self::Anxious),
            "surprised" | "surprise" | "curious" => Ok(Self::Surprised),
            "calm" => Ok(Self::Calm),
            "excited" => Ok(Self::Excited),
            _ => Err(DomainError::InvalidEmotion(raw.into())),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Happy => "happy",
            Self::Sad => "sad",
            Self::Angry => "angry",
            Self::Anxious => "anxious",
            Self::Surprised => "surprised",
            Self::Calm => "calm",
            Self::Excited => "excited",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_canonical_values() {
        for v in [
            "neutral",
            "happy",
            "sad",
            "angry",
            "anxious",
            "surprised",
            "calm",
            "excited",
        ] {
            assert!(EmotionalTone::parse(v).is_ok(), "{v}");
        }
    }

    #[test]
    fn parse_maps_legacy_synonyms() {
        assert_eq!(
            EmotionalTone::parse("Frustrated").unwrap(),
            EmotionalTone::Angry
        );
        assert_eq!(
            EmotionalTone::parse("worried").unwrap(),
            EmotionalTone::Anxious
        );
        assert_eq!(
            EmotionalTone::parse("curious").unwrap(),
            EmotionalTone::Surprised
        );
    }

    #[test]
    fn parse_rejects_unknown() {
        assert!(matches!(
            EmotionalTone::parse("melancholic"),
            Err(DomainError::InvalidEmotion(_))
        ));
    }
}
