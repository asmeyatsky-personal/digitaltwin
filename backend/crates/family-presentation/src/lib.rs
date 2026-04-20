//! Presentation (Family bounded context). REST + MCP.

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
use family_application::{
    AddMember, AddMemberInput, CreateFamily, CreateFamilyInput, GetFamily, ListFamiliesForUser,
    ListMembers,
};
use family_domain::{FamilyRole, UserRef};
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{str::FromStr, sync::Arc};

#[derive(Clone)]
pub struct FamilyServices {
    pub create: Arc<CreateFamily>,
    pub add_member: Arc<AddMember>,
    pub get: Arc<GetFamily>,
    pub list_members: Arc<ListMembers>,
    pub list_for_user: Arc<ListFamiliesForUser>,
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

pub fn router(s: FamilyServices) -> Router {
    Router::new()
        .route("/v1/families", post(create))
        .route("/v1/families/{family_id}", get(get_family))
        .route(
            "/v1/families/{family_id}/members",
            post(add_member).get(list_members),
        )
        .route("/v1/users/{user_id}/families", get(list_for_user))
        .with_state(s)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct CreateBody {
    name: String,
    created_by_user_id: String,
}

async fn create(
    State(s): State<FamilyServices>,
    Json(b): Json<CreateBody>,
) -> Result<Json<Value>, ApiError> {
    let created_by = EntityId::<UserRef>::from_str(&b.created_by_user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let id = s
        .create
        .execute(CreateFamilyInput {
            name: b.name,
            created_by,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "family_id": id.to_string() })))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct AddMemberBody {
    user_id: String,
    role: String,
}

async fn add_member(
    State(s): State<FamilyServices>,
    Path(family_id): Path<String>,
    Json(b): Json<AddMemberBody>,
) -> Result<StatusCode, ApiError> {
    let family_id = EntityId::from_str(&family_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad family_id".into()))?;
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let role =
        FamilyRole::parse(&b.role).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    s.add_member
        .execute(AddMemberInput {
            family_id,
            user_id,
            role,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_family(
    State(s): State<FamilyServices>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let id =
        EntityId::from_str(&id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad id".into()))?;
    let f = s
        .get
        .execute(id)
        .await
        .map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(json!({
        "id": f.id.to_string(), "name": f.name,
        "created_by": f.created_by.to_string(), "created_at": f.created_at.to_rfc3339(),
    })))
}

async fn list_members(
    State(s): State<FamilyServices>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let id =
        EntityId::from_str(&id).map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad id".into()))?;
    let members = s
        .list_members
        .execute(id)
        .await
        .map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    let items: Vec<Value> = members
        .iter()
        .map(|m| {
            json!({
                "user_id": m.user_id.to_string(), "role": m.role.as_str(),
                "joined_at": m.joined_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(json!({ "members": items })))
}

async fn list_for_user(
    State(s): State<FamilyServices>,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let families = s
        .list_for_user
        .execute(user_id)
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let items: Vec<Value> = families
        .iter()
        .map(|f| {
            json!({
                "id": f.id.to_string(), "name": f.name,
                "created_by": f.created_by.to_string(), "created_at": f.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(json!({ "families": items })))
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

    pub struct FamilyMcp {
        s: FamilyServices,
    }
    impl FamilyMcp {
        pub fn new(s: FamilyServices) -> Self {
            Self { s }
        }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {"name": "digitaltwin-family", "version": env!("CARGO_PKG_VERSION")},
                        "capabilities": {"tools": {}, "resources": {}}
                    }),
                ),
                "tools/list" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "tools": [
                            {"name": "family.create", "description": "Create a family (WRITE).",
                             "inputSchema": {"type":"object","required":["name","created_by"],
                                              "properties":{"name":{"type":"string"},"created_by":{"type":"string"}}}},
                            {"name": "family.add_member", "description": "Add a member to a family (WRITE).",
                             "inputSchema": {"type":"object","required":["family_id","user_id","role"],
                                              "properties":{"family_id":{"type":"string"},"user_id":{"type":"string"},
                                                            "role":{"type":"string","enum":["owner","adult","child"]}}}}
                        ]
                    }),
                ),
                "resources/list" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "resources": [{"uri": "family://{family_id}/members", "name": "family_members",
                                       "description": "Members of a family (READ).", "mimeType": "application/json"}]
                    }),
                ),
                "tools/call" => {
                    let name = req
                        .params
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    match name {
                        "family.create" => {
                            let name_s = args
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string();
                            let Ok(created_by) = EntityId::<UserRef>::from_str(
                                args.get("created_by").and_then(Value::as_str).unwrap_or(""),
                            ) else {
                                return JsonRpcResponse::err(id, -32602, "bad created_by");
                            };
                            match self
                                .s
                                .create
                                .execute(CreateFamilyInput {
                                    name: name_s,
                                    created_by,
                                    actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
                                })
                                .await
                            {
                                Ok(fid) => JsonRpcResponse::ok(
                                    id,
                                    json!({"content": [{"type":"text","text": fid.to_string()}]}),
                                ),
                                Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                            }
                        }
                        "family.add_member" => {
                            let Ok(family_id) = EntityId::from_str(
                                args.get("family_id").and_then(Value::as_str).unwrap_or(""),
                            ) else {
                                return JsonRpcResponse::err(id, -32602, "bad family_id");
                            };
                            let Ok(user_id) = EntityId::<UserRef>::from_str(
                                args.get("user_id").and_then(Value::as_str).unwrap_or(""),
                            ) else {
                                return JsonRpcResponse::err(id, -32602, "bad user_id");
                            };
                            let Ok(role) = FamilyRole::parse(
                                args.get("role").and_then(Value::as_str).unwrap_or(""),
                            ) else {
                                return JsonRpcResponse::err(id, -32602, "bad role");
                            };
                            match self
                                .s
                                .add_member
                                .execute(AddMemberInput {
                                    family_id,
                                    user_id,
                                    role,
                                    actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
                                })
                                .await
                            {
                                Ok(()) => JsonRpcResponse::ok(
                                    id,
                                    json!({"content":[{"type":"text","text":"added"}]}),
                                ),
                                Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                            }
                        }
                        other => JsonRpcResponse::err(id, -32602, format!("unknown tool: {other}")),
                    }
                }
                "resources/read" => {
                    let uri = req
                        .params
                        .get("uri")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let Some(rest) = uri.strip_prefix("family://") else {
                        return JsonRpcResponse::err(id, -32602, "unknown");
                    };
                    let Some((fid_s, _)) = rest.split_once("/members") else {
                        return JsonRpcResponse::err(id, -32602, "only /members supported");
                    };
                    let Ok(fid) = EntityId::from_str(fid_s) else {
                        return JsonRpcResponse::err(id, -32602, "bad family_id");
                    };
                    match self.s.list_members.execute(fid).await {
                        Ok(members) => JsonRpcResponse::ok(
                            id,
                            json!({
                                "contents": [{
                                    "uri": uri,
                                    "mimeType": "application/json",
                                    "text": serde_json::to_string(&members.iter().map(|m| json!({
                                        "user_id": m.user_id.to_string(), "role": m.role.as_str(),
                                        "joined_at": m.joined_at.to_rfc3339()
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
