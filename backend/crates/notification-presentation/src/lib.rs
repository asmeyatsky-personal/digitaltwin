//! Presentation (Notification). REST + MCP.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::Actor;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, post},
};
use kernel::EntityId;
use notification_application::{
    RegisterDevice, RegisterDeviceInput, SendPush, SendPushInput, UnregisterDevice,
};
use notification_domain::{Platform, UserRef};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{str::FromStr, sync::Arc};

#[derive(Clone)]
pub struct NotificationServices {
    pub register: Arc<RegisterDevice>,
    pub unregister: Arc<UnregisterDevice>,
    pub send: Arc<SendPush>,
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

pub fn router(s: NotificationServices) -> Router {
    Router::new()
        .route("/v1/devices", post(register))
        .route("/v1/devices/{token}", delete(unregister))
        .route("/v1/push", post(send))
        .with_state(s)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct RegisterBody {
    user_id: String,
    token: String,
    platform: String,
}

async fn register(
    State(s): State<NotificationServices>,
    Json(b): Json<RegisterBody>,
) -> Result<StatusCode, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let platform = Platform::parse(&b.platform)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    s.register
        .execute(RegisterDeviceInput {
            user_id,
            token: b.token,
            platform,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn unregister(
    State(s): State<NotificationServices>,
    axum::extract::Path(token): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    s.unregister
        .execute(&token)
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct PushBody {
    user_id: String,
    title: String,
    body: String,
    #[serde(default)]
    data: Value,
}

async fn send(
    State(s): State<NotificationServices>,
    Json(b): Json<PushBody>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let n = s
        .send
        .execute(SendPushInput {
            user_id,
            title: b.title,
            body: b.body,
            data: b.data,
        })
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    Ok(Json(json!({ "delivered": n })))
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
    pub struct NotificationMcp {
        s: NotificationServices,
    }
    impl NotificationMcp {
        pub fn new(s: NotificationServices) -> Self {
            Self { s }
        }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "protocolVersion":"2024-11-05",
                        "serverInfo":{"name":"digitaltwin-notification","version":env!("CARGO_PKG_VERSION")},
                        "capabilities":{"tools":{},"resources":{}}
                    }),
                ),
                "tools/list" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "tools":[{"name":"notification.push","description":"Send a push notification (WRITE).",
                            "inputSchema":{"type":"object","required":["user_id","title","body"],
                                "properties":{"user_id":{"type":"string"},"title":{"type":"string"},"body":{"type":"string"},
                                    "data":{"type":"object"}}}}]
                    }),
                ),
                "resources/list" => JsonRpcResponse::ok(id, json!({ "resources": [] })),
                "tools/call" => {
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    let name = req
                        .params
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if name != "notification.push" {
                        return JsonRpcResponse::err(id, -32602, format!("unknown tool: {name}"));
                    }
                    let Ok(user_id) = EntityId::<UserRef>::from_str(
                        args.get("user_id").and_then(Value::as_str).unwrap_or(""),
                    ) else {
                        return JsonRpcResponse::err(id, -32602, "bad user_id");
                    };
                    let title = args
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let body = args
                        .get("body")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let data = args.get("data").cloned().unwrap_or(json!({}));
                    match self
                        .s
                        .send
                        .execute(SendPushInput {
                            user_id,
                            title,
                            body,
                            data,
                        })
                        .await
                    {
                        Ok(n) => JsonRpcResponse::ok(
                            id,
                            json!({"content":[{"type":"text","text":format!("delivered {n}")}]}),
                        ),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                _ => JsonRpcResponse::err(id, -32601, "method not found"),
            }
        }
    }
}
