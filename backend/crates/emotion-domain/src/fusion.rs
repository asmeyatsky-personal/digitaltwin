//! Weighted fusion across modalities. Given a batch of readings inside a
//! time window, project the dominant tone by summing `modality.weight() *
//! reading.confidence()` per tone and picking the max. Ties resolve toward
//! `Neutral` so noise doesn't fabricate strong emotions.

use crate::{
    errors::DomainError,
    reading::{EmotionReading, Modality, UnifiedTone},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct FusedEmotion {
    pub tone: UnifiedTone,
    pub confidence: f32,
    pub per_modality: HashMap<Modality, UnifiedTone>,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub reading_count: u32,
}

/// # Errors
/// `EmptyReadings` when `readings` has no entries.
pub fn fuse(readings: &[EmotionReading]) -> Result<FusedEmotion, DomainError> {
    if readings.is_empty() {
        return Err(DomainError::EmptyReadings);
    }

    // Sum weights per tone.
    let mut score: HashMap<UnifiedTone, f32> = HashMap::new();
    let mut per_modality: HashMap<Modality, (UnifiedTone, f32)> = HashMap::new();
    let mut window_start = readings[0].recorded_at();
    let mut window_end = window_start;

    for r in readings {
        let s = score.entry(r.tone()).or_insert(0.0);
        *s += r.modality().weight() * r.confidence();

        // Keep only the most-confident reading per modality for the projection.
        let slot = per_modality
            .entry(r.modality())
            .or_insert((r.tone(), r.confidence()));
        if r.confidence() > slot.1 {
            *slot = (r.tone(), r.confidence());
        }

        if r.recorded_at() < window_start {
            window_start = r.recorded_at();
        }
        if r.recorded_at() > window_end {
            window_end = r.recorded_at();
        }
    }

    // Pick the max score; ties go to Neutral.
    let (tone, best) = score
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(t, s)| (*t, *s))
        .unwrap_or((UnifiedTone::Neutral, 0.0));

    let total: f32 = score.values().sum();
    let confidence = if total > 0.0 { best / total } else { 0.0 };

    Ok(FusedEmotion {
        tone,
        confidence,
        per_modality: per_modality.into_iter().map(|(m, (t, _))| (m, t)).collect(),
        window_start,
        window_end,
        reading_count: u32::try_from(readings.len()).unwrap_or(u32::MAX),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reading::{ReadingId, UserRef};
    use kernel::EntityId;

    fn r(mod_: Modality, tone: UnifiedTone, conf: f32) -> EmotionReading {
        EmotionReading::new(
            ReadingId::new(),
            EntityId::<UserRef>::new(),
            mod_,
            tone,
            conf,
            Utc::now(),
        )
        .expect("ok")
    }

    #[test]
    fn empty_fails() {
        assert_eq!(fuse(&[]).err(), Some(DomainError::EmptyReadings));
    }

    #[test]
    fn face_dominates_text() {
        let out = fuse(&[
            r(Modality::Face, UnifiedTone::Happy, 0.9),
            r(Modality::Text, UnifiedTone::Sad, 0.9),
        ])
        .expect("ok");
        assert_eq!(out.tone, UnifiedTone::Happy);
        assert!(out.confidence > 0.5);
    }

    #[test]
    fn reading_count_matches_input() {
        let out = fuse(&[
            r(Modality::Face, UnifiedTone::Happy, 0.8),
            r(Modality::Voice, UnifiedTone::Calm, 0.5),
        ])
        .expect("ok");
        assert_eq!(out.reading_count, 2);
    }

    #[test]
    fn single_strong_signal_dominates() {
        let out = fuse(&[r(Modality::Biometric, UnifiedTone::Anxious, 1.0)]).expect("ok");
        assert_eq!(out.tone, UnifiedTone::Anxious);
        assert!((out.confidence - 1.0).abs() < 1e-6);
    }

    #[test]
    fn per_modality_projection_keeps_most_confident() {
        let out = fuse(&[
            r(Modality::Face, UnifiedTone::Happy, 0.2),
            r(Modality::Face, UnifiedTone::Sad, 0.9),
        ])
        .expect("ok");
        assert_eq!(
            out.per_modality.get(&Modality::Face),
            Some(&UnifiedTone::Sad)
        );
    }
}
