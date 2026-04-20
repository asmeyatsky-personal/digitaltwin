use crate::errors::DomainError;
use chrono::{DateTime, Utc};
use kernel::EntityId;
use serde::Serialize;

/// Unified 8-tone taxonomy, identical to `conversation-domain::EmotionalTone`
/// but duplicated here so the Emotion context has no cross-context dep. The
/// adapter boundary is responsible for mapping between proto enums of each
/// context; the two enums stay bit-compatible by contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedTone {
    Neutral,
    Happy,
    Sad,
    Angry,
    Anxious,
    Surprised,
    Calm,
    Excited,
}

impl UnifiedTone {
    /// # Errors
    /// `UnknownTone` for unsupported values.
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
            other => Err(DomainError::UnknownTone(other.into())),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Face,
    Voice,
    Text,
    Biometric,
}

impl Modality {
    /// # Errors
    /// `UnknownModality` for unsupported inputs.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_lowercase().as_str() {
            "face" => Ok(Self::Face),
            "voice" => Ok(Self::Voice),
            "text" => Ok(Self::Text),
            "biometric" => Ok(Self::Biometric),
            other => Err(DomainError::UnknownModality(other.into())),
        }
    }

    /// Weight of this modality during fusion. Grounded in empirical observation
    /// that facial expression is the strongest non-verbal cue, followed by
    /// vocal tone, text semantics, and physiological signals (which are noisy
    /// at consumer hardware precision).
    #[must_use]
    pub const fn weight(self) -> f32 {
        match self {
            Self::Face => 0.40,
            Self::Voice => 0.30,
            Self::Text => 0.20,
            Self::Biometric => 0.10,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Face => "face",
            Self::Voice => "voice",
            Self::Text => "text",
            Self::Biometric => "biometric",
        }
    }
}

pub struct ReadingIdMarker;
pub type ReadingId = EntityId<ReadingIdMarker>;

#[derive(Debug, Clone, Serialize)]
pub struct EmotionReading {
    id: ReadingId,
    user_id: kernel::EntityId<UserRef>,
    modality: Modality,
    tone: UnifiedTone,
    /// 0.0 = no signal, 1.0 = full confidence.
    confidence: f32,
    recorded_at: DateTime<Utc>,
}

pub struct UserRef;

impl EmotionReading {
    /// # Errors
    /// `InvalidConfidence` when the confidence is not in [0.0, 1.0].
    pub fn new(
        id: ReadingId,
        user_id: kernel::EntityId<UserRef>,
        modality: Modality,
        tone: UnifiedTone,
        confidence: f32,
        recorded_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if !(0.0..=1.0).contains(&confidence) || confidence.is_nan() {
            return Err(DomainError::InvalidConfidence(confidence.to_string()));
        }
        Ok(Self {
            id,
            user_id,
            modality,
            tone,
            confidence,
            recorded_at,
        })
    }

    #[must_use]
    pub fn id(&self) -> ReadingId {
        self.id
    }
    #[must_use]
    pub fn user_id(&self) -> kernel::EntityId<UserRef> {
        self.user_id
    }
    #[must_use]
    pub fn modality(&self) -> Modality {
        self.modality
    }
    #[must_use]
    pub fn tone(&self) -> UnifiedTone {
        self.tone
    }
    #[must_use]
    pub fn confidence(&self) -> f32 {
        self.confidence
    }
    #[must_use]
    pub fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel::EntityId;

    #[test]
    fn rejects_out_of_range_confidence() {
        let e = EmotionReading::new(
            ReadingId::new(),
            EntityId::<UserRef>::new(),
            Modality::Face,
            UnifiedTone::Happy,
            1.5,
            Utc::now(),
        )
        .err();
        assert!(matches!(e, Some(DomainError::InvalidConfidence(_))));
    }

    #[test]
    fn rejects_nan_confidence() {
        let e = EmotionReading::new(
            ReadingId::new(),
            EntityId::<UserRef>::new(),
            Modality::Face,
            UnifiedTone::Happy,
            f32::NAN,
            Utc::now(),
        )
        .err();
        assert!(matches!(e, Some(DomainError::InvalidConfidence(_))));
    }

    #[test]
    fn accepts_boundary_confidences() {
        for c in [0.0, 0.5, 1.0] {
            assert!(
                EmotionReading::new(
                    ReadingId::new(),
                    EntityId::<UserRef>::new(),
                    Modality::Voice,
                    UnifiedTone::Calm,
                    c,
                    Utc::now()
                )
                .is_ok()
            );
        }
    }

    #[test]
    fn parse_tone_maps_legacy_strings() {
        assert_eq!(
            UnifiedTone::parse("Frustrated").unwrap(),
            UnifiedTone::Angry
        );
        assert_eq!(UnifiedTone::parse("fear").unwrap(), UnifiedTone::Anxious);
    }

    #[test]
    fn parse_modality_rejects_unknown() {
        assert!(matches!(
            Modality::parse("telepathy"),
            Err(DomainError::UnknownModality(_))
        ));
    }

    #[test]
    fn modality_weights_sum_to_one() {
        let total = Modality::Face.weight()
            + Modality::Voice.weight()
            + Modality::Text.weight()
            + Modality::Biometric.weight();
        assert!((total - 1.0).abs() < 1e-6);
    }
}
