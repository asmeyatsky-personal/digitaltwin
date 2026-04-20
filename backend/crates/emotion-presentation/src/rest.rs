use crate::EmotionServices;
use audit::Actor;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use emotion_application::{GetTimelineInput, ReportReadingInput};
use emotion_domain::{Modality, UnifiedTone, reading::UserRef};
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub fn router(services: EmotionServices) -> Router {
    Router::new()
        .route("/v1/readings", post(report))
        .route("/v1/users/{user_id}/current", get(current))
        .route("/v1/users/{user_id}/timeline", get(timeline))
        .with_state(services)
}

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body {
            error: String,
        }
        (self.0, Json(Body { error: self.1 })).into_response()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct ReportBody {
    user_id: String,
    modality: String,
    tone: String,
    confidence: f32,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ReportResponse {
    reading_id: String,
}

async fn report(
    State(s): State<EmotionServices>,
    Json(b): Json<ReportBody>,
) -> Result<Json<ReportResponse>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let modality = Modality::parse(&b.modality)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let tone = UnifiedTone::parse(&b.tone)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let out = s
        .report
        .execute(ReportReadingInput {
            user_id,
            modality,
            tone,
            confidence: b.confidence,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(ReportResponse {
        reading_id: out.reading_id.to_string(),
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CurrentResponse {
    tone: String,
    confidence: f32,
    reading_count: u32,
}

async fn current(
    State(s): State<EmotionServices>,
    Path(user_id): Path<String>,
) -> Result<Json<CurrentResponse>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let out = s
        .current
        .execute(user_id)
        .await
        .map_err(|e| ApiError(StatusCode::NOT_FOUND, format!("{e:?}")))?;
    Ok(Json(CurrentResponse {
        tone: out.fused.tone.as_str().into(),
        confidence: out.fused.confidence,
        reading_count: out.fused.reading_count,
    }))
}

#[derive(Deserialize)]
struct TimelineQuery {
    from: Option<String>,
    to: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct TimelineReading {
    id: String,
    modality: String,
    tone: String,
    confidence: f32,
    recorded_at: String,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct TimelineResponse {
    readings: Vec<TimelineReading>,
}

async fn timeline(
    State(s): State<EmotionServices>,
    Path(user_id): Path<String>,
    Query(q): Query<TimelineQuery>,
) -> Result<Json<TimelineResponse>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let from = q
        .from
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::days(7));
    let to = q
        .to
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);
    let out = s
        .timeline
        .execute(GetTimelineInput { user_id, from, to })
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, format!("{e:?}")))?;
    Ok(Json(TimelineResponse {
        readings: out
            .readings
            .iter()
            .map(|r| TimelineReading {
                id: r.id().to_string(),
                modality: r.modality().as_str().into(),
                tone: r.tone().as_str().into(),
                confidence: r.confidence(),
                recorded_at: r.recorded_at().to_rfc3339(),
            })
            .collect(),
    }))
}
