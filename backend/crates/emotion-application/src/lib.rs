//! Layer: application (Emotion bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

pub mod use_cases;

pub use use_cases::{
    CurrentEmotion, CurrentEmotionError, FuseCurrent, GetTimeline, GetTimelineError,
    GetTimelineInput, GetTimelineOutput, ReportReading, ReportReadingError, ReportReadingInput,
    ReportReadingOutput,
};
