//! Presentation (Learning). REST + MCP.

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
use learning_application::{
    CompleteModule, CompleteModuleInput, CreatePath, CreatePathInput, GetProgress, ListPaths,
    StartPath, StartPathInput,
};
use learning_domain::UserRef;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{str::FromStr, sync::Arc};

#[derive(Clone)]
pub struct LearningServices {
    pub create: Arc<CreatePath>,
    pub list: Arc<ListPaths>,
    pub start: Arc<StartPath>,
    pub complete: Arc<CompleteModule>,
    pub progress: Arc<GetProgress>,
}

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)] struct B { error: String }
        (self.0, Json(B { error: self.1 })).into_response()
    }
}

pub fn router(s: LearningServices) -> Router {
    Router::new()
        .route("/v1/paths", post(create_path).get(list_paths))
        .route("/v1/paths/{path_id}/start", post(start))
        .route("/v1/paths/{path_id}/complete-module", post(complete))
        .route("/v1/users/{user_id}/progress", get(progress))
        .with_state(s)
}

#[derive(Deserialize)] #[serde(rename_all="snake_case")]
struct CreateBody { title: String, #[serde(default)] description: String, category: String, modules: Vec<String>, estimated_minutes: u32 }

async fn create_path(State(s): State<LearningServices>, Json(b): Json<CreateBody>) -> Result<Json<Value>, ApiError> {
    let id = s.create.execute(CreatePathInput {
        title: b.title, description: b.description, category: b.category,
        modules: b.modules, estimated_minutes: b.estimated_minutes,
    }).await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "path_id": id.to_string() })))
}

#[derive(Deserialize)]
struct CategoryQuery { category: Option<String> }
async fn list_paths(State(s): State<LearningServices>, Query(q): Query<CategoryQuery>) -> Result<Json<Value>, ApiError> {
    let paths = s.list.execute(q.category).await.map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let arr: Vec<Value> = paths.iter().map(|p| json!({
        "id": p.id.to_string(), "title": p.title, "description": p.description,
        "category": p.category, "modules": p.modules, "estimated_minutes": p.estimated_minutes,
    })).collect();
    Ok(Json(json!({ "paths": arr })))
}

#[derive(Deserialize)] #[serde(rename_all="snake_case")]
struct StartBody { user_id: String }
async fn start(State(s): State<LearningServices>, Path(path_id): Path<String>, Json(b): Json<StartBody>) -> Result<StatusCode, ApiError> {
    let path_id = EntityId::from_str(&path_id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad path_id".into()))?;
    let user_id = EntityId::<UserRef>::from_str(&b.user_id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    s.start.execute(StartPathInput { user_id, path_id, actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()) })
        .await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)] #[serde(rename_all="snake_case")]
struct CompleteBody { user_id: String, #[serde(default)] reflection_notes: String }
async fn complete(State(s): State<LearningServices>, Path(path_id): Path<String>, Json(b): Json<CompleteBody>) -> Result<Json<Value>, ApiError> {
    let path_id = EntityId::from_str(&path_id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad path_id".into()))?;
    let user_id = EntityId::<UserRef>::from_str(&b.user_id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let completed = s.complete.execute(CompleteModuleInput {
        user_id, path_id, reflection_notes: b.reflection_notes,
        actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
    }).await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "completed": completed })))
}

async fn progress(State(s): State<LearningServices>, Path(user_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let items = s.progress.execute(user_id).await.map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let arr: Vec<Value> = items.iter().map(|p| json!({
        "path_id": p.path_id.to_string(), "current_module": p.current_module,
        "reflection_notes": p.reflection_notes, "started_at": p.started_at.to_rfc3339(),
        "completed_at": p.completed_at.map(|t| t.to_rfc3339()),
    })).collect();
    Ok(Json(json!({ "progress": arr })))
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
    pub struct LearningMcp { s: LearningServices }
    impl LearningMcp {
        pub fn new(s: LearningServices) -> Self { Self { s } }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(id, json!({
                    "protocolVersion":"2024-11-05",
                    "serverInfo":{"name":"digitaltwin-learning","version":env!("CARGO_PKG_VERSION")},
                    "capabilities":{"tools":{},"resources":{}}
                })),
                "tools/list" => JsonRpcResponse::ok(id, json!({
                    "tools":[{"name":"learning.complete_module","description":"Mark the current module complete and record a reflection (WRITE).",
                        "inputSchema":{"type":"object","required":["user_id","path_id"],
                            "properties":{"user_id":{"type":"string"},"path_id":{"type":"string"},
                                "reflection_notes":{"type":"string"}}}}]
                })),
                "resources/list" => JsonRpcResponse::ok(id, json!({
                    "resources":[{"uri":"learning://paths","name":"learning_paths",
                        "description":"All learning paths (READ).","mimeType":"application/json"}]
                })),
                "tools/call" => {
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    if req.params.get("name").and_then(Value::as_str) != Some("learning.complete_module") {
                        return JsonRpcResponse::err(id, -32602, "unknown tool");
                    }
                    let Ok(user_id) = EntityId::<UserRef>::from_str(args.get("user_id").and_then(Value::as_str).unwrap_or("")) else {
                        return JsonRpcResponse::err(id, -32602, "bad user_id");
                    };
                    let Ok(path_id) = EntityId::from_str(args.get("path_id").and_then(Value::as_str).unwrap_or("")) else {
                        return JsonRpcResponse::err(id, -32602, "bad path_id");
                    };
                    let notes = args.get("reflection_notes").and_then(Value::as_str).unwrap_or("").to_string();
                    match self.s.complete.execute(CompleteModuleInput {
                        user_id, path_id, reflection_notes: notes,
                        actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
                    }).await {
                        Ok(done) => JsonRpcResponse::ok(id, json!({"content":[{"type":"text","text": if done {"path complete"} else {"module advanced"}}]})),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                "resources/read" => {
                    match self.s.list.execute(None).await {
                        Ok(paths) => JsonRpcResponse::ok(id, json!({
                            "contents":[{"uri":"learning://paths","mimeType":"application/json",
                                "text": serde_json::to_string(&paths.iter().map(|p| json!({
                                    "id": p.id.to_string(), "title": p.title, "category": p.category,
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
