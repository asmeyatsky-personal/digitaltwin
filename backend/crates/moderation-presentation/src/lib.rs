//! Presentation (Moderation). REST + MCP.

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
use moderation_application::{ListPending, ReportContent, ReportContentInput, ReviewReport, ReviewReportInput};
use moderation_domain::{Reason, Status, UserRef};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{str::FromStr, sync::Arc};

#[derive(Clone)]
pub struct ModerationServices {
    pub report: Arc<ReportContent>,
    pub review: Arc<ReviewReport>,
    pub pending: Arc<ListPending>,
}

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)] struct B { error: String }
        (self.0, Json(B { error: self.1 })).into_response()
    }
}

pub fn router(s: ModerationServices) -> Router {
    Router::new()
        .route("/v1/reports", post(report).get(list_pending))
        .route("/v1/reports/{report_id}/review", post(review))
        .with_state(s)
}

#[derive(Deserialize)] #[serde(rename_all="snake_case")]
struct ReportBody { reporter: String, content_type: String, content_id: String, reason: String }

async fn report(State(s): State<ModerationServices>, Json(b): Json<ReportBody>) -> Result<Json<Value>, ApiError> {
    let reporter = EntityId::<UserRef>::from_str(&b.reporter).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad reporter".into()))?;
    let reason = Reason::parse(&b.reason).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let id = s.report.execute(ReportContentInput {
        reporter, content_type: b.content_type, content_id: b.content_id, reason,
        actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
    }).await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "report_id": id.to_string() })))
}

#[derive(Deserialize)] #[serde(rename_all="snake_case")]
struct ReviewBody { reviewer: String, status: String, #[serde(default)] notes: Option<String> }

async fn review(State(s): State<ModerationServices>, Path(report_id): Path<String>, Json(b): Json<ReviewBody>) -> Result<StatusCode, ApiError> {
    let report_id = EntityId::from_str(&report_id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad report_id".into()))?;
    let reviewer = EntityId::<UserRef>::from_str(&b.reviewer).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad reviewer".into()))?;
    let status = Status::parse(&b.status).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    s.review.execute(ReviewReportInput {
        report_id, reviewer, status, notes: b.notes,
        actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
    }).await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct LimitQuery { limit: Option<u32> }

async fn list_pending(State(s): State<ModerationServices>, Query(q): Query<LimitQuery>) -> Result<Json<Value>, ApiError> {
    let list = s.pending.execute(q.limit.unwrap_or(50)).await.map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let arr: Vec<Value> = list.iter().map(|r| json!({
        "id": r.id.to_string(), "reporter": r.reporter.to_string(),
        "content_type": r.content_type, "content_id": r.content_id,
        "reason": r.reason.as_str(), "status": r.status.as_str(),
        "notes": r.notes, "created_at": r.created_at.to_rfc3339(),
    })).collect();
    Ok(Json(json!({ "reports": arr })))
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
    pub struct ModerationMcp { s: ModerationServices }
    impl ModerationMcp {
        pub fn new(s: ModerationServices) -> Self { Self { s } }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(id, json!({
                    "protocolVersion":"2024-11-05",
                    "serverInfo":{"name":"digitaltwin-moderation","version":env!("CARGO_PKG_VERSION")},
                    "capabilities":{"tools":{},"resources":{}}
                })),
                "tools/list" => JsonRpcResponse::ok(id, json!({
                    "tools":[{"name":"moderation.report","description":"Report content for moderation (WRITE).",
                        "inputSchema":{"type":"object","required":["reporter","content_type","content_id","reason"],
                            "properties":{"reporter":{"type":"string"},"content_type":{"type":"string"},
                                "content_id":{"type":"string"},
                                "reason":{"type":"string","enum":["harassment","spam","self_harm","inappropriate","other"]}}}}]
                })),
                "resources/list" => JsonRpcResponse::ok(id, json!({
                    "resources":[{"uri":"moderation://pending","name":"pending_reports",
                        "description":"Pending moderation reports (READ).","mimeType":"application/json"}]
                })),
                "tools/call" => {
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    if req.params.get("name").and_then(Value::as_str) != Some("moderation.report") {
                        return JsonRpcResponse::err(id, -32602, "unknown tool");
                    }
                    let Ok(reporter) = EntityId::<UserRef>::from_str(args.get("reporter").and_then(Value::as_str).unwrap_or("")) else {
                        return JsonRpcResponse::err(id, -32602, "bad reporter");
                    };
                    let content_type = args.get("content_type").and_then(Value::as_str).unwrap_or("").to_string();
                    let content_id = args.get("content_id").and_then(Value::as_str).unwrap_or("").to_string();
                    let Ok(reason) = Reason::parse(args.get("reason").and_then(Value::as_str).unwrap_or("")) else {
                        return JsonRpcResponse::err(id, -32602, "bad reason");
                    };
                    match self.s.report.execute(ReportContentInput {
                        reporter, content_type, content_id, reason,
                        actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
                    }).await {
                        Ok(rid) => JsonRpcResponse::ok(id, json!({"content":[{"type":"text","text": rid.to_string()}]})),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                "resources/read" => {
                    match self.s.pending.execute(50).await {
                        Ok(list) => JsonRpcResponse::ok(id, json!({
                            "contents":[{"uri":"moderation://pending","mimeType":"application/json",
                                "text": serde_json::to_string(&list.iter().map(|r| json!({
                                    "id": r.id.to_string(), "reason": r.reason.as_str(),
                                    "content_type": r.content_type, "content_id": r.content_id,
                                })).collect::<Vec<_>>()).unwrap_or_default()
                            }]
                        })),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                _ => JsonRpcResponse::err(id, -32601, "method not found"),
            }
        }
    }
}
