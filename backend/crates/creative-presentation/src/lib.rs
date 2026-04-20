//! Presentation (Creative). REST + MCP.

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
use creative_application::{
    CreateWork, CreateWorkInput, GetWork, ListWorks, ListWorksInput, ShareWork,
};
use creative_domain::{UserRef, WorkType};
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{str::FromStr, sync::Arc};

#[derive(Clone)]
pub struct CreativeServices {
    pub create: Arc<CreateWork>,
    pub share: Arc<ShareWork>,
    pub get: Arc<GetWork>,
    pub list: Arc<ListWorks>,
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

pub fn router(s: CreativeServices) -> Router {
    Router::new()
        .route("/v1/works", post(create))
        .route("/v1/works/{id}", get(get_work))
        .route("/v1/works/{id}/share", post(share))
        .route("/v1/users/{user_id}/works", get(list_for_user))
        .with_state(s)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct CreateBody {
    user_id: String,
    work_type: String,
    title: String,
    content: String,
    #[serde(default)]
    mood: String,
}
async fn create(
    State(s): State<CreativeServices>,
    Json(b): Json<CreateBody>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let work_type = WorkType::parse(&b.work_type)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let id = s
        .create
        .execute(CreateWorkInput {
            user_id,
            work_type,
            title: b.title,
            content: b.content,
            mood: b.mood,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "work_id": id.to_string() })))
}

async fn get_work(
    State(s): State<CreativeServices>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let id =
        EntityId::from_str(&id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad id".into()))?;
    let w = s
        .get
        .execute(id)
        .await
        .map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(json!({
        "id": w.id.to_string(), "work_type": w.work_type.as_str(),
        "title": w.title, "content": w.content, "mood": w.mood,
        "is_shared": w.is_shared, "created_at": w.created_at.to_rfc3339(),
    })))
}

async fn share(
    State(s): State<CreativeServices>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id =
        EntityId::from_str(&id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad id".into()))?;
    s.share
        .execute(id)
        .await
        .map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct ListQuery {
    work_type: Option<String>,
    limit: Option<u32>,
}
async fn list_for_user(
    State(s): State<CreativeServices>,
    Path(user_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let work_type = q
        .work_type
        .as_deref()
        .map(WorkType::parse)
        .transpose()
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let items = s
        .list
        .execute(ListWorksInput {
            user_id,
            work_type,
            limit: q.limit.unwrap_or(20),
        })
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let arr: Vec<Value> = items
        .iter()
        .map(|w| {
            json!({
                "id": w.id.to_string(), "work_type": w.work_type.as_str(),
                "title": w.title, "mood": w.mood, "is_shared": w.is_shared,
                "created_at": w.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(json!({ "works": arr })))
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
    pub struct CreativeMcp {
        s: CreativeServices,
    }
    impl CreativeMcp {
        pub fn new(s: CreativeServices) -> Self {
            Self { s }
        }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "protocolVersion":"2024-11-05",
                        "serverInfo":{"name":"digitaltwin-creative","version":env!("CARGO_PKG_VERSION")},
                        "capabilities":{"tools":{},"resources":{}}
                    }),
                ),
                "tools/list" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "tools":[{"name":"creative.create_work","description":"Create a creative work (story/poem/reflection/gratitude) (WRITE).",
                            "inputSchema":{"type":"object","required":["user_id","work_type","title","content"],
                                "properties":{"user_id":{"type":"string"},
                                    "work_type":{"type":"string","enum":["story","poem","reflection","gratitude","other"]},
                                    "title":{"type":"string","minLength":1},"content":{"type":"string","minLength":1},
                                    "mood":{"type":"string"}}}}]
                    }),
                ),
                "resources/list" => JsonRpcResponse::ok(id, json!({ "resources": [] })),
                "tools/call" => {
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    if req.params.get("name").and_then(Value::as_str)
                        != Some("creative.create_work")
                    {
                        return JsonRpcResponse::err(id, -32602, "unknown tool");
                    }
                    let Ok(user_id) = EntityId::<UserRef>::from_str(
                        args.get("user_id").and_then(Value::as_str).unwrap_or(""),
                    ) else {
                        return JsonRpcResponse::err(id, -32602, "bad user_id");
                    };
                    let Ok(work_type) = WorkType::parse(
                        args.get("work_type").and_then(Value::as_str).unwrap_or(""),
                    ) else {
                        return JsonRpcResponse::err(id, -32602, "bad work_type");
                    };
                    let title = args
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let content = args
                        .get("content")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let mood = args
                        .get("mood")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    match self
                        .s
                        .create
                        .execute(CreateWorkInput {
                            user_id,
                            work_type,
                            title,
                            content,
                            mood,
                            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
                        })
                        .await
                    {
                        Ok(id2) => JsonRpcResponse::ok(
                            id,
                            json!({"content":[{"type":"text","text": id2.to_string()}]}),
                        ),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                _ => JsonRpcResponse::err(id, -32601, "method not found"),
            }
        }
    }
}
