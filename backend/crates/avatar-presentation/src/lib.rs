//! Presentation (Avatar). REST + MCP.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::Actor;
use avatar_application::{GenerateAvatar, GenerateAvatarInput, GetJob, ListJobsForUser};
use avatar_domain::{GenerationJob, UserRef};
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

#[derive(Clone)]
pub struct AvatarServices {
    pub generate: Arc<GenerateAvatar>,
    pub get: Arc<GetJob>,
    pub list: Arc<ListJobsForUser>,
}

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)] struct B { error: String }
        (self.0, Json(B { error: self.1 })).into_response()
    }
}

fn job_to_json(j: &GenerationJob) -> Value {
    json!({
        "id": j.id.to_string(),
        "status": j.status.as_str(),
        "result_url": j.result_url,
        "failure_reason": j.failure_reason,
        "created_at": j.created_at.to_rfc3339(),
        "completed_at": j.completed_at.map(|t| t.to_rfc3339()),
    })
}

pub fn router(s: AvatarServices) -> Router {
    Router::new()
        .route("/v1/jobs", post(generate))
        .route("/v1/jobs/{job_id}", get(get_job))
        .route("/v1/users/{user_id}/jobs", get(list_for_user))
        .with_state(s)
}

#[derive(Deserialize)] #[serde(rename_all="snake_case")]
struct GenBody { user_id: String, photo_url: String }

async fn generate(State(s): State<AvatarServices>, Json(b): Json<GenBody>) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let id = s.generate.execute(GenerateAvatarInput {
        user_id, photo_url: b.photo_url, actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
    }).await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "job_id": id.to_string() })))
}

async fn get_job(State(s): State<AvatarServices>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let id = EntityId::from_str(&id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad id".into()))?;
    let j = s.get.execute(id).await.map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(job_to_json(&j)))
}

async fn list_for_user(State(s): State<AvatarServices>, Path(user_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let jobs = s.list.execute(user_id).await.map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    Ok(Json(json!({ "jobs": jobs.iter().map(job_to_json).collect::<Vec<_>>() })))
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
    pub struct AvatarMcp { s: AvatarServices }
    impl AvatarMcp {
        pub fn new(s: AvatarServices) -> Self { Self { s } }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(id, json!({
                    "protocolVersion":"2024-11-05",
                    "serverInfo":{"name":"digitaltwin-avatar","version":env!("CARGO_PKG_VERSION")},
                    "capabilities":{"tools":{},"resources":{}}
                })),
                "tools/list" => JsonRpcResponse::ok(id, json!({
                    "tools": [{"name":"avatar.generate","description":"Generate a 3D avatar from a photo (WRITE).",
                        "inputSchema":{"type":"object","required":["user_id","photo_url"],
                            "properties":{"user_id":{"type":"string"},"photo_url":{"type":"string","format":"uri"}}}}]
                })),
                "resources/list" => JsonRpcResponse::ok(id, json!({
                    "resources":[{"uri":"avatar://jobs/{job_id}","name":"avatar_job",
                        "description":"Avatar generation job status (READ).","mimeType":"application/json"}]
                })),
                "tools/call" => {
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    let name = req.params.get("name").and_then(Value::as_str).unwrap_or_default();
                    if name != "avatar.generate" {
                        return JsonRpcResponse::err(id, -32602, format!("unknown tool: {name}"));
                    }
                    let Ok(user_id) = EntityId::<UserRef>::from_str(args.get("user_id").and_then(Value::as_str).unwrap_or("")) else {
                        return JsonRpcResponse::err(id, -32602, "bad user_id");
                    };
                    let photo_url = args.get("photo_url").and_then(Value::as_str).unwrap_or("").to_string();
                    match self.s.generate.execute(GenerateAvatarInput {
                        user_id, photo_url, actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
                    }).await {
                        Ok(jid) => JsonRpcResponse::ok(id, json!({"content":[{"type":"text","text": jid.to_string()}]})),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                _ => JsonRpcResponse::err(id, -32601, "method not found"),
            }
        }
    }
}
