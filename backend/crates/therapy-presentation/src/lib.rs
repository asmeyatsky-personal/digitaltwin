//! Presentation (Therapy). REST + MCP.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::Actor;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{str::FromStr, sync::Arc};
use therapy_application::{
    ListScreeningsForUser, ListTherapists, RegisterTherapist, RegisterTherapistInput,
    SubmitScreening, SubmitScreeningInput,
};
use therapy_domain::{ScreeningType, UserRef};

#[derive(Clone)]
pub struct TherapyServices {
    pub register: Arc<RegisterTherapist>,
    pub list_t: Arc<ListTherapists>,
    pub submit: Arc<SubmitScreening>,
    pub list_s: Arc<ListScreeningsForUser>,
}

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)] struct B { error: String }
        (self.0, Json(B { error: self.1 })).into_response()
    }
}

pub fn router(s: TherapyServices) -> Router {
    Router::new()
        .route("/v1/therapists", post(register).get(list_therapists))
        .route("/v1/screenings", post(submit))
        .route("/v1/users/{user_id}/screenings", get(list_screenings))
        .with_state(s)
}

#[derive(Deserialize)] #[serde(rename_all="snake_case")]
struct RegBody { name: String, #[serde(default)] credentials: String, #[serde(default)] specializations: Vec<String>, rate_per_session: u32 }

async fn register(State(s): State<TherapyServices>, Json(b): Json<RegBody>) -> Result<Json<Value>, ApiError> {
    let id = s.register.execute(RegisterTherapistInput {
        name: b.name, credentials: b.credentials, specializations: b.specializations, rate_per_session: b.rate_per_session,
        actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
    }).await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "therapist_id": id.to_string() })))
}

#[derive(Deserialize)]
struct LimitQuery { limit: Option<u32> }

async fn list_therapists(State(s): State<TherapyServices>, Query(q): Query<LimitQuery>) -> Result<Json<Value>, ApiError> {
    let list = s.list_t.execute(q.limit.unwrap_or(20)).await.map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let arr: Vec<Value> = list.iter().map(|t| json!({
        "id": t.id.to_string(), "name": t.name, "credentials": t.credentials,
        "specializations": t.specializations, "rate_per_session": t.rate_per_session,
        "is_verified": t.is_verified,
    })).collect();
    Ok(Json(json!({ "therapists": arr })))
}

#[derive(Deserialize)] #[serde(rename_all="snake_case")]
struct SubmitBody { user_id: String, screening_type: String, responses: Vec<u8> }

async fn submit(State(s): State<TherapyServices>, Json(b): Json<SubmitBody>) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let screening_type = ScreeningType::parse(&b.screening_type).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let out = s.submit.execute(SubmitScreeningInput {
        user_id, screening_type, responses: b.responses,
        actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
    }).await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({
        "screening_id": out.screening_id.to_string(),
        "score": out.score, "severity": out.severity.as_str(),
    })))
}

async fn list_screenings(State(s): State<TherapyServices>, Path(user_id): Path<String>, Query(q): Query<LimitQuery>) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let list = s.list_s.execute(user_id, q.limit.unwrap_or(20)).await.map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let arr: Vec<Value> = list.iter().map(|r| json!({
        "id": r.id.to_string(), "screening_type": r.screening_type.as_str(),
        "score": r.score, "severity": r.severity.as_str(),
        "completed_at": r.completed_at.to_rfc3339(),
    })).collect();
    Ok(Json(json!({ "screenings": arr })))
}

pub mod mcp {
    use super::*;
    #[derive(Debug, Deserialize)]
    pub struct JsonRpcRequest { pub jsonrpc: String, pub id: Option<Value>, pub method: String, #[serde(default)] pub params: Value }
    #[derive(Debug, Serialize)]
    pub struct JsonRpcResponse {
        pub jsonrpc: &'static str, pub id: Option<Value>,
        #[serde(skip_serializing_if="Option::is_none")] pub result: Option<Value>,
        #[serde(skip_serializing_if="Option::is_none")] pub error: Option<JsonRpcError>,
    }
    #[derive(Debug, Serialize)] pub struct JsonRpcError { pub code: i32, pub message: String }
    impl JsonRpcResponse {
        fn ok(id: Option<Value>, r: Value) -> Self { Self { jsonrpc:"2.0", id, result: Some(r), error: None } }
        fn err(id: Option<Value>, c: i32, m: impl Into<String>) -> Self {
            Self { jsonrpc:"2.0", id, result: None, error: Some(JsonRpcError { code:c, message: m.into() }) }
        }
    }
    pub struct TherapyMcp { s: TherapyServices }
    impl TherapyMcp {
        pub fn new(s: TherapyServices) -> Self { Self { s } }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(id, json!({
                    "protocolVersion":"2024-11-05",
                    "serverInfo":{"name":"digitaltwin-therapy","version":env!("CARGO_PKG_VERSION")},
                    "capabilities":{"tools":{},"resources":{}}
                })),
                "tools/list" => JsonRpcResponse::ok(id, json!({
                    "tools":[{"name":"therapy.submit_screening","description":"Submit a PHQ-9 or GAD-7 screening (WRITE).",
                        "inputSchema":{"type":"object","required":["user_id","screening_type","responses"],
                            "properties":{"user_id":{"type":"string"},
                                "screening_type":{"type":"string","enum":["PHQ9","GAD7"]},
                                "responses":{"type":"array","items":{"type":"integer","minimum":0,"maximum":3}}}}}]
                })),
                "resources/list" => JsonRpcResponse::ok(id, json!({
                    "resources":[{"uri":"therapy://therapists","name":"therapists",
                        "description":"Available therapists (READ).","mimeType":"application/json"}]
                })),
                "tools/call" => {
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    if req.params.get("name").and_then(Value::as_str) != Some("therapy.submit_screening") {
                        return JsonRpcResponse::err(id, -32602, "unknown tool");
                    }
                    let Ok(user_id) = EntityId::<UserRef>::from_str(args.get("user_id").and_then(Value::as_str).unwrap_or("")) else {
                        return JsonRpcResponse::err(id, -32602, "bad user_id");
                    };
                    let Ok(screening_type) = ScreeningType::parse(args.get("screening_type").and_then(Value::as_str).unwrap_or("")) else {
                        return JsonRpcResponse::err(id, -32602, "bad type");
                    };
                    let responses: Vec<u8> = args.get("responses").and_then(Value::as_array)
                        .map(|a| a.iter().filter_map(|v| v.as_u64().map(|x| x as u8)).collect())
                        .unwrap_or_default();
                    match self.s.submit.execute(SubmitScreeningInput {
                        user_id, screening_type, responses,
                        actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
                    }).await {
                        Ok(out) => JsonRpcResponse::ok(id, json!({
                            "content":[{"type":"text","text": format!("score={} severity={}", out.score, out.severity.as_str())}]
                        })),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                "resources/read" => {
                    match self.s.list_t.execute(50).await {
                        Ok(list) => JsonRpcResponse::ok(id, json!({
                            "contents":[{"uri":"therapy://therapists","mimeType":"application/json",
                                "text": serde_json::to_string(&list.iter().map(|t| json!({
                                    "id": t.id.to_string(), "name": t.name, "verified": t.is_verified,
                                })).collect::<Vec<_>>()).unwrap_or_default()}]
                        })),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                _ => JsonRpcResponse::err(id, -32601, "method not found"),
            }
        }
    }
}
