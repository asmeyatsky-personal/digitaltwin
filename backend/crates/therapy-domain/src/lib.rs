//! Domain (Therapy bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use chrono::{DateTime, Utc};
use kernel::EntityId;
use serde::Serialize;
use thiserror::Error;

pub struct UserRef;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("name cannot be empty")]
    EmptyName,
    #[error("unknown screening type: {0}")]
    UnknownScreening(String),
    #[error("invalid PHQ-9 response: expected {expected} items with scores 0-3")]
    BadPhq9 { expected: u32 },
    #[error("invalid GAD-7 response: expected {expected} items with scores 0-3")]
    BadGad7 { expected: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ScreeningType {
    Phq9,
    Gad7,
}
impl ScreeningType {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.to_uppercase().as_str() {
            "PHQ9" | "PHQ-9" => Ok(Self::Phq9),
            "GAD7" | "GAD-7" => Ok(Self::Gad7),
            other => Err(DomainError::UnknownScreening(other.into())),
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Phq9 => "PHQ9",
            Self::Gad7 => "GAD7",
        }
    }
    pub fn expected_items(self) -> u32 {
        match self {
            Self::Phq9 => 9,
            Self::Gad7 => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    None,
    Mild,
    Moderate,
    ModeratelySevere,
    Severe,
}
impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Mild => "mild",
            Self::Moderate => "moderate",
            Self::ModeratelySevere => "moderately_severe",
            Self::Severe => "severe",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TherapistProfile {
    pub id: EntityId<TherapistProfile>,
    pub name: String,
    pub credentials: String,
    pub specializations: Vec<String>,
    pub rate_per_session: u32,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
}
impl TherapistProfile {
    pub fn new(
        id: EntityId<TherapistProfile>,
        name: String,
        credentials: String,
        specializations: Vec<String>,
        rate_per_session: u32,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id,
            name,
            credentials,
            specializations,
            rate_per_session,
            is_verified: false,
            created_at: now,
        })
    }
    pub fn verify(&self) -> Self {
        Self {
            is_verified: true,
            ..self.clone()
        }
    }
}

/// Score a PHQ-9 / GAD-7 screening. Both are 0-3 Likert items; PHQ-9 has 9
/// items (max 27), GAD-7 has 7 (max 21). Severity cut-points are the
/// clinically standard APA thresholds.
#[derive(Debug, Clone, Serialize)]
pub struct ClinicalScreening {
    pub id: EntityId<ClinicalScreening>,
    pub user_id: EntityId<UserRef>,
    pub screening_type: ScreeningType,
    pub responses: Vec<u8>,
    pub score: u32,
    pub severity: Severity,
    pub completed_at: DateTime<Utc>,
}
impl ClinicalScreening {
    pub fn submit(
        id: EntityId<ClinicalScreening>,
        user_id: EntityId<UserRef>,
        screening_type: ScreeningType,
        responses: Vec<u8>,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if u32::try_from(responses.len()).unwrap_or(u32::MAX) != screening_type.expected_items()
            || responses.iter().any(|r| *r > 3)
        {
            return Err(match screening_type {
                ScreeningType::Phq9 => DomainError::BadPhq9 { expected: 9 },
                ScreeningType::Gad7 => DomainError::BadGad7 { expected: 7 },
            });
        }
        let score: u32 = responses.iter().map(|r| u32::from(*r)).sum();
        let severity = match (screening_type, score) {
            (ScreeningType::Phq9, s) if s <= 4 => Severity::None,
            (ScreeningType::Phq9, s) if s <= 9 => Severity::Mild,
            (ScreeningType::Phq9, s) if s <= 14 => Severity::Moderate,
            (ScreeningType::Phq9, s) if s <= 19 => Severity::ModeratelySevere,
            (ScreeningType::Phq9, _) => Severity::Severe,
            (ScreeningType::Gad7, s) if s <= 4 => Severity::None,
            (ScreeningType::Gad7, s) if s <= 9 => Severity::Mild,
            (ScreeningType::Gad7, s) if s <= 14 => Severity::Moderate,
            (ScreeningType::Gad7, _) => Severity::Severe,
        };
        Ok(Self {
            id,
            user_id,
            screening_type,
            responses,
            score,
            severity,
            completed_at: now,
        })
    }
}

#[async_trait::async_trait]
pub trait TherapyRepository: Send + Sync {
    async fn save_therapist(&self, t: &TherapistProfile) -> Result<(), StoreError>;
    async fn list_therapists(&self, limit: u32) -> Result<Vec<TherapistProfile>, StoreError>;
    async fn save_screening(&self, s: &ClinicalScreening) -> Result<(), StoreError>;
    async fn screenings_for_user(
        &self,
        user_id: EntityId<UserRef>,
        limit: u32,
    ) -> Result<Vec<ClinicalScreening>, StoreError>;
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("backend: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phq9_severe_threshold() {
        let s = ClinicalScreening::submit(
            EntityId::new(),
            EntityId::new(),
            ScreeningType::Phq9,
            vec![3, 3, 3, 3, 3, 3, 3, 1, 1],
            Utc::now(),
        )
        .unwrap();
        assert_eq!(s.score, 23);
        assert_eq!(s.severity, Severity::Severe);
    }
    #[test]
    fn gad7_minimal() {
        let s = ClinicalScreening::submit(
            EntityId::new(),
            EntityId::new(),
            ScreeningType::Gad7,
            vec![0, 1, 0, 1, 0, 0, 1],
            Utc::now(),
        )
        .unwrap();
        assert_eq!(s.score, 3);
        assert_eq!(s.severity, Severity::None);
    }
    #[test]
    fn rejects_wrong_item_count() {
        assert!(matches!(
            ClinicalScreening::submit(
                EntityId::new(),
                EntityId::new(),
                ScreeningType::Phq9,
                vec![0; 8],
                Utc::now()
            ),
            Err(DomainError::BadPhq9 { expected: 9 })
        ));
    }
    #[test]
    fn rejects_out_of_range_responses() {
        assert!(
            ClinicalScreening::submit(
                EntityId::new(),
                EntityId::new(),
                ScreeningType::Gad7,
                vec![4, 0, 0, 0, 0, 0, 0],
                Utc::now()
            )
            .is_err()
        );
    }
}
