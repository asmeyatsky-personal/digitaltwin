use crate::EmotionServices;
use audit::Actor;
use emotion_application::{GetTimelineInput, ReportReadingInput};
use emotion_contracts::v1::{
    GetCurrentRequest, GetCurrentResponse, GetTimelineRequest, GetTimelineResponse,
    Modality as ProtoModality, Reading as ProtoReading, ReportReadingRequest,
    ReportReadingResponse, Tone as ProtoTone,
    emotion_service_server::{EmotionService, EmotionServiceServer},
};
use emotion_domain::{EmotionReading, Modality, UnifiedTone, reading::UserRef};
use kernel::EntityId;
use prost_types::Timestamp;
use std::str::FromStr;
use tonic::{Request, Response, Status};

pub struct EmotionGrpc {
    services: EmotionServices,
}

impl EmotionGrpc {
    #[must_use]
    pub fn new(services: EmotionServices) -> EmotionServiceServer<Self> {
        EmotionServiceServer::new(Self { services })
    }
}

fn ts(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: i32::try_from(dt.timestamp_subsec_nanos()).unwrap_or(0),
    }
}
fn from_ts(t: &Timestamp) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(t.seconds, t.nanos.try_into().unwrap_or(0)).unwrap_or_default()
}
fn tone_to_proto(t: UnifiedTone) -> i32 {
    match t {
        UnifiedTone::Neutral => ProtoTone::Neutral as i32,
        UnifiedTone::Happy => ProtoTone::Happy as i32,
        UnifiedTone::Sad => ProtoTone::Sad as i32,
        UnifiedTone::Angry => ProtoTone::Angry as i32,
        UnifiedTone::Anxious => ProtoTone::Anxious as i32,
        UnifiedTone::Surprised => ProtoTone::Surprised as i32,
        UnifiedTone::Calm => ProtoTone::Calm as i32,
        UnifiedTone::Excited => ProtoTone::Excited as i32,
    }
}
fn tone_from_proto(v: i32) -> UnifiedTone {
    match v {
        x if x == ProtoTone::Happy as i32 => UnifiedTone::Happy,
        x if x == ProtoTone::Sad as i32 => UnifiedTone::Sad,
        x if x == ProtoTone::Angry as i32 => UnifiedTone::Angry,
        x if x == ProtoTone::Anxious as i32 => UnifiedTone::Anxious,
        x if x == ProtoTone::Surprised as i32 => UnifiedTone::Surprised,
        x if x == ProtoTone::Calm as i32 => UnifiedTone::Calm,
        x if x == ProtoTone::Excited as i32 => UnifiedTone::Excited,
        _ => UnifiedTone::Neutral,
    }
}
fn modality_to_proto(m: Modality) -> i32 {
    match m {
        Modality::Face => ProtoModality::Face as i32,
        Modality::Voice => ProtoModality::Voice as i32,
        Modality::Text => ProtoModality::Text as i32,
        Modality::Biometric => ProtoModality::Biometric as i32,
    }
}
fn modality_from_proto(v: i32) -> Option<Modality> {
    match v {
        x if x == ProtoModality::Face as i32 => Some(Modality::Face),
        x if x == ProtoModality::Voice as i32 => Some(Modality::Voice),
        x if x == ProtoModality::Text as i32 => Some(Modality::Text),
        x if x == ProtoModality::Biometric as i32 => Some(Modality::Biometric),
        _ => None,
    }
}
fn reading_to_proto(r: &EmotionReading) -> ProtoReading {
    ProtoReading {
        id: r.id().to_string(),
        modality: modality_to_proto(r.modality()),
        tone: tone_to_proto(r.tone()),
        confidence: r.confidence(),
        recorded_at: Some(ts(r.recorded_at())),
    }
}

#[tonic::async_trait]
impl EmotionService for EmotionGrpc {
    async fn report_reading(
        &self,
        request: Request<ReportReadingRequest>,
    ) -> Result<Response<ReportReadingResponse>, Status> {
        let req = request.into_inner();
        let user_id = EntityId::<UserRef>::from_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("bad user_id"))?;
        let modality = modality_from_proto(req.modality)
            .ok_or_else(|| Status::invalid_argument("modality required"))?;
        let tone = tone_from_proto(req.tone);
        let out = self
            .services
            .report
            .execute(ReportReadingInput {
                user_id,
                modality,
                tone,
                confidence: req.confidence,
                actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
            })
            .await
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        Ok(Response::new(ReportReadingResponse {
            reading_id: out.reading_id.to_string(),
        }))
    }

    async fn get_current(
        &self,
        request: Request<GetCurrentRequest>,
    ) -> Result<Response<GetCurrentResponse>, Status> {
        let req = request.into_inner();
        let user_id = EntityId::<UserRef>::from_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("bad user_id"))?;
        let out = self
            .services
            .current
            .execute(user_id)
            .await
            .map_err(|e| Status::unavailable(format!("{e:?}")))?;
        Ok(Response::new(GetCurrentResponse {
            tone: tone_to_proto(out.fused.tone),
            confidence: out.fused.confidence,
            reading_count: out.fused.reading_count,
            window_start: Some(ts(out.fused.window_start)),
            window_end: Some(ts(out.fused.window_end)),
        }))
    }

    async fn get_timeline(
        &self,
        request: Request<GetTimelineRequest>,
    ) -> Result<Response<GetTimelineResponse>, Status> {
        let req = request.into_inner();
        let user_id = EntityId::<UserRef>::from_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("bad user_id"))?;
        let from = req.from.as_ref().map(from_ts).unwrap_or_default();
        let to = req
            .to
            .as_ref()
            .map(from_ts)
            .unwrap_or_else(chrono::Utc::now);
        let out = self
            .services
            .timeline
            .execute(GetTimelineInput { user_id, from, to })
            .await
            .map_err(|e| Status::unavailable(format!("{e:?}")))?;
        Ok(Response::new(GetTimelineResponse {
            readings: out.readings.iter().map(reading_to_proto).collect(),
        }))
    }
}
