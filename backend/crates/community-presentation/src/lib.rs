//! Presentation (Community). REST + MCP.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use audit::Actor;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use community_application::{
    CreateGroup, CreateGroupInput, CreatePost, CreatePostInput, JoinGroup, ListGroups, ListPosts,
};
use community_domain::UserRef;
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{str::FromStr, sync::Arc};

#[derive(Clone)]
pub struct CommunityServices {
    pub create_group: Arc<CreateGroup>,
    pub list_groups: Arc<ListGroups>,
    pub join: Arc<JoinGroup>,
    pub create_post: Arc<CreatePost>,
    pub list_posts: Arc<ListPosts>,
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

pub fn router(s: CommunityServices) -> Router {
    Router::new()
        .route("/v1/groups", post(create_group).get(list_groups))
        .route("/v1/groups/{group_id}/join", post(join))
        .route(
            "/v1/groups/{group_id}/posts",
            post(create_post).get(list_posts),
        )
        .with_state(s)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct CreateGroupBody {
    name: String,
    #[serde(default)]
    description: String,
    category: String,
    #[serde(default = "default_true")]
    is_moderated: bool,
    created_by: String,
}
fn default_true() -> bool {
    true
}

async fn create_group(
    State(s): State<CommunityServices>,
    Json(b): Json<CreateGroupBody>,
) -> Result<Json<Value>, ApiError> {
    let created_by = EntityId::<UserRef>::from_str(&b.created_by)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    let id = s
        .create_group
        .execute(CreateGroupInput {
            name: b.name,
            description: b.description,
            category: b.category,
            is_moderated: b.is_moderated,
            created_by,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "group_id": id.to_string() })))
}

#[derive(Deserialize)]
struct ListGroupsQuery {
    category: Option<String>,
    limit: Option<u32>,
}

async fn list_groups(
    State(s): State<CommunityServices>,
    Query(q): Query<ListGroupsQuery>,
) -> Result<Json<Value>, ApiError> {
    let groups = s
        .list_groups
        .execute(q.category, q.limit.unwrap_or(20))
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let arr: Vec<Value> = groups
        .iter()
        .map(|g| {
            json!({
                "id": g.id.to_string(), "name": g.name, "description": g.description,
                "category": g.category, "is_moderated": g.is_moderated,
                "created_by": g.created_by.to_string(), "created_at": g.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(json!({ "groups": arr })))
}

#[derive(Deserialize)]
struct JoinBody {
    user_id: String,
}
async fn join(
    State(s): State<CommunityServices>,
    Path(group_id): Path<String>,
    Json(b): Json<JoinBody>,
) -> Result<StatusCode, ApiError> {
    let group_id = EntityId::from_str(&group_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad group_id".into()))?;
    let user_id = EntityId::<UserRef>::from_str(&b.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad user_id".into()))?;
    s.join
        .execute(group_id, user_id)
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct CreatePostBody {
    author: String,
    content: String,
    #[serde(default)]
    is_anonymous: bool,
}

async fn create_post(
    State(s): State<CommunityServices>,
    Path(group_id): Path<String>,
    Json(b): Json<CreatePostBody>,
) -> Result<Json<Value>, ApiError> {
    let group_id = EntityId::from_str(&group_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad group_id".into()))?;
    let author = EntityId::<UserRef>::from_str(&b.author)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad author".into()))?;
    let id = s
        .create_post
        .execute(CreatePostInput {
            group_id,
            author,
            content: b.content,
            is_anonymous: b.is_anonymous,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "post_id": id.to_string() })))
}

#[derive(Deserialize)]
struct LimitQuery {
    limit: Option<u32>,
}
async fn list_posts(
    State(s): State<CommunityServices>,
    Path(group_id): Path<String>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Value>, ApiError> {
    let group_id = EntityId::from_str(&group_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "bad group_id".into()))?;
    let posts = s
        .list_posts
        .execute(group_id, q.limit.unwrap_or(50))
        .await
        .map_err(|e| ApiError(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    let arr: Vec<Value> = posts.iter().map(|p| json!({
        "id": p.id.to_string(), "group_id": p.group_id.to_string(),
        "author": if p.is_anonymous { Value::Null } else { Value::String(p.author.to_string()) },
        "content": p.content, "is_anonymous": p.is_anonymous,
        "created_at": p.created_at.to_rfc3339(),
    })).collect();
    Ok(Json(json!({ "posts": arr })))
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
    pub struct CommunityMcp {
        s: CommunityServices,
    }
    impl CommunityMcp {
        pub fn new(s: CommunityServices) -> Self {
            Self { s }
        }
        pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
            let id = req.id.clone();
            match req.method.as_str() {
                "initialize" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "protocolVersion":"2024-11-05",
                        "serverInfo":{"name":"digitaltwin-community","version":env!("CARGO_PKG_VERSION")},
                        "capabilities":{"tools":{},"resources":{}}
                    }),
                ),
                "tools/list" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "tools":[{"name":"community.post","description":"Create a post in a community group (WRITE).",
                            "inputSchema":{"type":"object","required":["group_id","author","content"],
                                "properties":{"group_id":{"type":"string"},"author":{"type":"string"},
                                    "content":{"type":"string","minLength":1},"is_anonymous":{"type":"boolean"}}}}]
                    }),
                ),
                "resources/list" => JsonRpcResponse::ok(
                    id,
                    json!({
                        "resources":[{"uri":"community://groups","name":"community_groups",
                            "description":"List all community groups (READ).","mimeType":"application/json"}]
                    }),
                ),
                "tools/call" => {
                    let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
                    if req.params.get("name").and_then(Value::as_str) != Some("community.post") {
                        return JsonRpcResponse::err(id, -32602, "unknown tool");
                    }
                    let Ok(group_id) = EntityId::from_str(
                        args.get("group_id").and_then(Value::as_str).unwrap_or(""),
                    ) else {
                        return JsonRpcResponse::err(id, -32602, "bad group_id");
                    };
                    let Ok(author) = EntityId::<UserRef>::from_str(
                        args.get("author").and_then(Value::as_str).unwrap_or(""),
                    ) else {
                        return JsonRpcResponse::err(id, -32602, "bad author");
                    };
                    let content = args
                        .get("content")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let is_anonymous = args
                        .get("is_anonymous")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    match self
                        .s
                        .create_post
                        .execute(CreatePostInput {
                            group_id,
                            author,
                            content,
                            is_anonymous,
                            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
                        })
                        .await
                    {
                        Ok(pid) => JsonRpcResponse::ok(
                            id,
                            json!({"content":[{"type":"text","text": pid.to_string()}]}),
                        ),
                        Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                    }
                }
                "resources/read" => match self.s.list_groups.execute(None, 50).await {
                    Ok(groups) => JsonRpcResponse::ok(
                        id,
                        json!({
                            "contents":[{"uri":"community://groups","mimeType":"application/json",
                                "text": serde_json::to_string(&groups.iter().map(|g| json!({
                                    "id": g.id.to_string(), "name": g.name, "category": g.category,
                                })).collect::<Vec<_>>()).unwrap_or_default()}]
                        }),
                    ),
                    Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                },
                _ => JsonRpcResponse::err(id, -32601, "method not found"),
            }
        }
    }
}
