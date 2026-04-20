//! Layer: presentation (Memory bounded context). REST + MCP.

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
use chrono::{DateTime, Utc};
use kernel::EntityId;
use memory_application::{
    AddLifeEvent, AddLifeEventInput, GetConversationContext, GetTimeline, GetUpcoming, RecordMemory,
    RecordMemoryInput,
};
use memory_domain::{LifeEventCategory, UserRef};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{str::FromStr, sync::Arc};

#[derive(Clone)]
pub struct MemoryServices {
    pub record: Arc<RecordMemory>,
    pub timeline: Arc<GetTimeline>,
    pub add_event: Arc<AddLifeEvent>,
    pub upcoming: Arc<GetUpcoming>,
    pub context: Arc<GetConversationContext>,
}

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body { error: String }
        (self.0, Json(Body { error: self.1 })).into_response()
    }
}

pub fn router(services: MemoryServices) -> Router {
    Router::new()
        .route("/v1/memories", post(record_memory))
        .route("/v1/users/{user_id}/timeline", get(timeline))
        .route("/v1/life-events", post(add_life_event))
        .route("/v1/users/{user_id}/upcoming", get(upcoming))
        .route("/v1/users/{user_id}/context", get(context))
        .with_state(services)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct RecordMemoryBody {
    user_id: String,
    content: String,
    mood: String,
    #[serde(default)]
    tags: Vec<String>,
}

async fn record_memory(
    State(s): State<MemoryServices>,
    Json(b): Json<RecordMemoryBody>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let out = s
        .record
        .execute(RecordMemoryInput {
            user_id,
            content: b.content,
            mood: b.mood,
            tags: b.tags,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "memory_id": out.memory_id.to_string() })))
}

#[derive(Deserialize)]
struct LimitQuery { limit: Option<u32> }

async fn timeline(
    State(s): State<MemoryServices>,
    Path(user_id): Path<String>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let memories = s
        .timeline
        .execute(user_id, q.limit.unwrap_or(20))
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let items: Vec<Value> = memories
        .iter()
        .map(|m| json!({
            "id": m.id.to_string(),
            "content": m.content,
            "mood": m.mood,
            "tags": m.tags,
            "created_at": m.created_at.to_rfc3339(),
        }))
        .collect();
    Ok(Json(json!({ "memories": items })))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct AddLifeEventBody {
    user_id: String,
    title: String,
    description: String,
    event_date: String,
    category: String,
    emotional_impact: i32,
    #[serde(default)]
    is_recurring: bool,
}

async fn add_life_event(
    State(s): State<MemoryServices>,
    Json(b): Json<AddLifeEventBody>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let event_date = DateTime::parse_from_rfc3339(&b.event_date)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad event_date".into()))?
        .with_timezone(&Utc);
    let category = LifeEventCategory::parse(&b.category)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let id = s
        .add_event
        .execute(AddLifeEventInput {
            user_id,
            title: b.title,
            description: b.description,
            event_date,
            category,
            emotional_impact: b.emotional_impact,
            is_recurring: b.is_recurring,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "event_id": id.to_string() })))
}

#[derive(Deserialize)]
struct HorizonQuery { horizon_days: Option<u32> }

async fn upcoming(
    State(s): State<MemoryServices>,
    Path(user_id): Path<String>,
    Query(q): Query<HorizonQuery>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let events = s
        .upcoming
        .execute(user_id, q.horizon_days.unwrap_or(30))
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let items: Vec<Value> = events
        .iter()
        .map(|e| json!({
            "id": e.id.to_string(),
            "title": e.title,
            "description": e.description,
            "event_date": e.event_date.to_rfc3339(),
            "category": e.category.as_str(),
            "emotional_impact": e.emotional_impact,
            "is_recurring": e.is_recurring,
        }))
        .collect();
    Ok(Json(json!({ "events": items })))
}

async fn context(
    State(s): State<MemoryServices>,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let text = s
        .context
        .execute(user_id)
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    Ok(Json(json!({ "context_text": text })))
}

// ---- MCP ----

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
        fn ok(id: Option<Value>, result: Value) -> Self {
            Self { jsonrpc: "2.0", id, result: Some(result), error: None }
        }
        fn err(id: Option<Value>, code: i32, msg: impl Into<String>) -> Self {
            Self { jsonrpc: "2.0", id, result: None, error: Some(JsonRpcError { code, message: msg.into() }) }
        }
    }

    pub struct MemoryMcp { services: MemoryServices }
    impl MemoryMcp {
        pub fn new(services: MemoryServices) -> Self { Self { services } }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(id, json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": { "name": "digitaltwin-memory", "version": env!("CARGO_PKG_VERSION") },
                    "capabilities": { "tools": {}, "resources": {} }
                })),
                "tools/list" => JsonRpcResponse::ok(id, json!({
                    "tools": [{
                        "name": "memory.record",
                        "description": "Record a memory for the user (WRITE).",
                        "inputSchema": {
                            "type": "object",
                            "required": ["user_id", "content"],
                            "properties": {
                                "user_id": {"type": "string"},
                                "content": {"type": "string", "minLength": 1},
                                "mood": {"type": "string"},
                                "tags": {"type": "array", "items": {"type": "string"}}
                            }
                        }
                    }]
                })),
                "resources/list" => JsonRpcResponse::ok(id, json!({
                    "resources": [
                        { "uri": "memory://{user_id}/timeline", "name": "memory_timeline",
                          "description": "User's recent memories (READ).", "mimeType": "application/json" },
                        { "uri": "memory://{user_id}/context",  "name": "conversation_context",
                          "description": "Compact context text for LLM prompts (READ).", "mimeType": "text/plain" }
                    ]
                })),
                "tools/call" => {
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    let name = req.params.get("name").and_then(Value::as_str).unwrap_or_default();
                    if name != "memory.record" {
                        return JsonRpcResponse::err(id, -32602, format!("unknown tool: {name}"));
                    }
                    let user_id_s = args.get("user_id").and_then(Value::as_str).unwrap_or_default();
                    let Ok(user_id) = EntityId::<UserRef>::from_str(user_id_s) else {
                        return JsonRpcResponse::err(id, -32602, "bad user_id");
                    };
                    let content = args.get("content").and_then(Value::as_str).unwrap_or("").to_string();
                    let mood = args.get("mood").and_then(Value::as_str).unwrap_or("").to_string();
                    let tags: Vec<String> = args.get("tags").and_then(Value::as_array)
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
                        .unwrap_or_default();
                    match self.services.record.execute(RecordMemoryInput {
                        user_id, content, mood, tags,
                        actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
                    }).await {
                        Ok(out) => JsonRpcResponse::ok(id, json!({
                            "content": [{ "type": "text", "text": out.memory_id.to_string() }]
                        })),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                "resources/read" => {
                    let uri = req.params.get("uri").and_then(Value::as_str).unwrap_or_default();
                    if let Some(rest) = uri.strip_prefix("memory://") {
                        if let Some((user_s, suffix)) = rest.split_once('/') {
                            let Ok(user_id) = EntityId::<UserRef>::from_str(user_s) else {
                                return JsonRpcResponse::err(id, -32602, "bad user");
                            };
                            match suffix {
                                "timeline" => match self.services.timeline.execute(user_id, 10).await {
                                    Ok(list) => JsonRpcResponse::ok(id, json!({
                                        "contents": [{
                                            "uri": uri,
                                            "mimeType": "application/json",
                                            "text": serde_json::to_string(&list.iter().map(|m| json!({
                                                "content": m.content, "mood": m.mood, "tags": m.tags,
                                                "created_at": m.created_at.to_rfc3339(),
                                            })).collect::<Vec<_>>()).unwrap_or_default(),
                                        }]
                                    })),
                                    Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                                },
                                "context" => match self.services.context.execute(user_id).await {
                                    Ok(text) => JsonRpcResponse::ok(id, json!({
                                        "contents": [{ "uri": uri, "mimeType": "text/plain", "text": text }]
                                    })),
                                    Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                                },
                                _ => JsonRpcResponse::err(id, -32602, "unknown resource"),
                            }
                        } else {
                            JsonRpcResponse::err(id, -32602, "malformed uri")
                        }
                    } else {
                        JsonRpcResponse::err(id, -32602, "unknown resource")
                    }
                }
                _ => JsonRpcResponse::err(id, -32601, "method not found"),
            }
        }
    }
}
