//! Presentation (Achievement bounded context). REST + MCP.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use achievement_application::{
    ListAchievements, ListForUser, RecordProgress, RecordProgressInput, UpsertAchievement,
};
use achievement_domain::UserRef;
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

#[derive(Clone)]
pub struct AchievementServices {
    pub upsert: Arc<UpsertAchievement>,
    pub record: Arc<RecordProgress>,
    pub list_all: Arc<ListAchievements>,
    pub list_for_user: Arc<ListForUser>,
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

pub fn router(s: AchievementServices) -> Router {
    Router::new()
        .route("/v1/achievements", get(list_all))
        .route("/v1/progress", post(record))
        .route("/v1/users/{user_id}/achievements", get(list_for_user))
        .with_state(s)
}

async fn list_all(State(s): State<AchievementServices>) -> Result<Json<Value>, ApiError> {
    let items = s
        .list_all
        .execute()
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let arr: Vec<Value> = items.iter().map(|a| json!({
        "id": a.id.to_string(), "key": a.key, "title": a.title,
        "description": a.description, "category": a.category, "required_count": a.required_count,
    })).collect();
    Ok(Json(json!({ "achievements": arr })))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct RecordBody {
    user_id: String,
    achievement_key: String,
    delta: u32,
}

async fn record(
    State(s): State<AchievementServices>,
    Json(b): Json<RecordBody>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let out = s
        .record
        .execute(RecordProgressInput {
            user_id,
            achievement_key: b.achievement_key,
            delta: b.delta,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(
        json!({ "progress": out.progress, "unlocked": out.unlocked }),
    ))
}

async fn list_for_user(
    State(s): State<AchievementServices>,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let items = s
        .list_for_user
        .execute(user_id)
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let arr: Vec<Value> = items
        .iter()
        .map(|u| {
            json!({
                "achievement_id": u.achievement_id.to_string(),
                "progress": u.progress,
                "unlocked_at": u.unlocked_at.map(|t| t.to_rfc3339()),
            })
        })
        .collect();
    Ok(Json(json!({ "achievements": arr })))
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
    pub struct AchievementMcp {
        s: AchievementServices,
    }
    impl AchievementMcp {
        pub fn new(s: AchievementServices) -> Self {
            Self { s }
        }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {"name":"digitaltwin-achievement", "version": env!("CARGO_PKG_VERSION")},
                        "capabilities": {"tools": {}, "resources": {}}
                    }),
                ),
                "tools/list" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "tools": [{"name":"achievement.record_progress","description":"Record progress toward an achievement (WRITE).",
                            "inputSchema": {"type":"object","required":["user_id","achievement_key","delta"],
                                "properties":{"user_id":{"type":"string"},"achievement_key":{"type":"string"},
                                              "delta":{"type":"integer","minimum":1}}}}]
                    }),
                ),
                "resources/list" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "resources": [{"uri":"achievement://{user_id}/unlocked","name":"unlocked_achievements",
                            "description":"User's achievement progress (READ).","mimeType":"application/json"}]
                    }),
                ),
                "tools/call" => {
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    let name = req
                        .params
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if name != "achievement.record_progress" {
                        return JsonRpcResponse::err(id, -32602, format!("unknown tool: {name}"));
                    }
                    let Ok(user_id) = EntityId::<UserRef>::from_str(
                        args.get("user_id").and_then(Value::as_str).unwrap_or(""),
                    ) else {
                        return JsonRpcResponse::err(id, -32602, "bad user_id");
                    };
                    let achievement_key = args
                        .get("achievement_key")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let delta = args.get("delta").and_then(Value::as_u64).unwrap_or(1) as u32;
                    match self
                        .s
                        .record
                        .execute(RecordProgressInput {
                            user_id,
                            achievement_key,
                            delta,
                            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
                        })
                        .await
                    {
                        Ok(out) => JsonRpcResponse::ok(
                            id,
                            json!({
                                "content":[{"type":"text","text": format!("progress={} unlocked={}", out.progress, out.unlocked)}]
                            }),
                        ),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                "resources/read" => {
                    let uri = req
                        .params
                        .get("uri")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let Some(rest) = uri.strip_prefix("achievement://") else {
                        return JsonRpcResponse::err(id, -32602, "unknown");
                    };
                    let user_s = rest.split('/').next().unwrap_or("");
                    let Ok(user_id) = EntityId::<UserRef>::from_str(user_s) else {
                        return JsonRpcResponse::err(id, -32602, "bad user_id");
                    };
                    match self.s.list_for_user.execute(user_id).await {
                        Ok(items) => JsonRpcResponse::ok(
                            id,
                            json!({
                                "contents":[{"uri":uri,"mimeType":"application/json",
                                    "text": serde_json::to_string(&items.iter().map(|u| json!({
                                        "achievement_id": u.achievement_id.to_string(), "progress": u.progress,
                                        "unlocked_at": u.unlocked_at.map(|t| t.to_rfc3339()),
                                    })).collect::<Vec<_>>()).unwrap_or_default()
                                }]
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
