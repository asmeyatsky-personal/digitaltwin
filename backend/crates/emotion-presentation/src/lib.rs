//! Layer: presentation (Emotion bounded context). gRPC + REST + MCP.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

pub mod grpc;
pub mod mcp;
pub mod rest;

use std::sync::Arc;

use emotion_application::{FuseCurrent, GetTimeline, ReportReading};

#[derive(Clone)]
pub struct EmotionServices {
    pub report: Arc<ReportReading>,
    pub current: Arc<FuseCurrent>,
    pub timeline: Arc<GetTimeline>,
}
