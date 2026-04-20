//! Presentation (Voice). REST + MCP.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::Actor;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{str::FromStr, sync::Arc};
use voice_application::{GetJob, RegisterVoice, RegisterVoiceInput, Synthesize, SynthesizeInput};
use voice_domain::UserRef;

#[derive(Clone)]
pub struct VoiceServices {
    pub register: Arc<RegisterVoice>,
    pub synthesize: Arc<Synthesize>,
    pub get_job: Arc<GetJob>,
}

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct B {
            error: String,
        }
        (self.0, Json(B { error: self.1 })).into_response()
    }
}

pub fn router(s: VoiceServices) -> Router {
    Router::new()
        .route("/v1/profiles", post(register))
        .route("/v1/synthesize", post(synthesize))
        .route("/v1/jobs/{job_id}", get(get_job))
        .with_state(s)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct RegBody {
    user_id: String,
    sample_url: String,
}
async fn register(
    State(s): State<VoiceServices>,
    Json(b): Json<RegBody>,
) -> Result<StatusCode, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    s.register
        .execute(RegisterVoiceInput {
            user_id,
            sample_url: b.sample_url,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct SynthBody {
    user_id: String,
    text: String,
    #[serde(default = "default_emotion")]
    emotion: String,
}
fn default_emotion() -> String {
    "neutral".into()
}
async fn synthesize(
    State(s): State<VoiceServices>,
    Json(b): Json<SynthBody>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let out = s
        .synthesize
        .execute(SynthesizeInput {
            user_id,
            text: b.text,
            emotion: b.emotion,
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(
        json!({ "job_id": out.job_id.to_string(), "audio_url": out.audio_url }),
    ))
}

async fn get_job(
    State(s): State<VoiceServices>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let id =
        EntityId::from_str(&id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad id".into()))?;
    let j = s
        .get_job
        .execute(id)
        .await
        .map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(json!({
        "status": j.status.as_str(), "audio_url": j.audio_url,
        "text": j.text, "emotion": j.emotion,
    })))
}

pub mod mcp {
    use super::*;
    #[derive(Debug, Deserialize)]
    pub struct JsonRpcRequest {
        pub jsonrpc: String,
        pub id: Option<Value>,
        pub method: String,
        #[serde(default)]
        pub params: Value,
    }
    #[derive(Debug, Serialize)]
    pub struct JsonRpcResponse {
        pub jsonrpc: &'static str,
        pub id: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub result: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<JsonRpcError>,
    }
    #[derive(Debug, Serialize)]
    pub struct JsonRpcError {
        pub code: i32,
        pub message: String,
    }
    impl JsonRpcResponse {
        fn ok(id: Option<Value>, r: Value) -> Self {
            Self {
                jsonrpc: "2.0",
                id,
                result: Some(r),
                error: None,
            }
        }
        fn err(id: Option<Value>, c: i32, m: impl Into<String>) -> Self {
            Self {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: c,
                    message: m.into(),
                }),
            }
        }
    }
    pub struct VoiceMcp {
        s: VoiceServices,
    }
    impl VoiceMcp {
        pub fn new(s: VoiceServices) -> Self {
            Self { s }
        }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "protocolVersion":"2024-11-05",
                        "serverInfo":{"name":"digitaltwin-voice","version":env!("CARGO_PKG_VERSION")},
                        "capabilities":{"tools":{},"resources":{}}
                    }),
                ),
                "tools/list" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "tools":[{"name":"voice.synthesize","description":"Synthesize text in the user's cloned voice (WRITE).",
                            "inputSchema":{"type":"object","required":["user_id","text"],
                                "properties":{"user_id":{"type":"string"},"text":{"type":"string","minLength":1},
                                    "emotion":{"type":"string"}}}}]
                    }),
                ),
                "resources/list" => JsonRpcResponse::ok(id, json!({ "resources": [] })),
                "tools/call" => {
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    if req.params.get("name").and_then(Value::as_str) != Some("voice.synthesize") {
                        return JsonRpcResponse::err(id, -32602, "unknown tool");
                    }
                    let Ok(user_id) = EntityId::<UserRef>::from_str(
                        args.get("user_id").and_then(Value::as_str).unwrap_or(""),
                    ) else {
                        return JsonRpcResponse::err(id, -32602, "bad user_id");
                    };
                    let text = args
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let emotion = args
                        .get("emotion")
                        .and_then(Value::as_str)
                        .unwrap_or("neutral")
                        .to_string();
                    match self
                        .s
                        .synthesize
                        .execute(SynthesizeInput {
                            user_id,
                            text,
                            emotion,
                        })
                        .await
                    {
                        Ok(out) => JsonRpcResponse::ok(
                            id,
                            json!({
                                "content":[{"type":"text","text": out.audio_url.unwrap_or_default()}]
                            }),
                        ),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                _ => JsonRpcResponse::err(id, -32601, "method not found"),
            }
        }
    }
}
