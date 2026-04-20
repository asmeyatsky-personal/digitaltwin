//! Layer: domain (Emotion bounded context).
//! Ports: `ReadingRepository`.
//! MCP integration: Emotion MCP server (tools = report_reading; resources =
//! current_state, timeline).
//! Stack choice: canonical.
//!
//! Multi-modal emotional signal fusion (AD-1). Each modality (face, voice,
//! text, biometric) contributes an `EmotionReading` with a confidence. The
//! `FusedEmotion` aggregate projects the weighted tone over a time window.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::module_name_repetitions)]

pub mod errors;
pub mod fusion;
pub mod ports;
pub mod reading;

pub use errors::DomainError;
pub use fusion::{FusedEmotion, fuse};
pub use reading::{EmotionReading, Modality, UnifiedTone};
