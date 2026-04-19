//! JSON REST adapter for the Conversation context. Mirrors the proto shape
//! in lower_snake_case so the generated TS types serialise correctly.

use crate::ConversationServices;
use audit::Actor;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use conversation_application::{
    EndConversationError, EndConversationInput, GetHistoryError, GetHistoryInput,
    ListConversationsError, ListConversationsInput, SendMessageError, SendMessageInput,
    StartConversationError, StartConversationInput,
};
use conversation_domain::{Conversation, Message, Role, conversation::UserRef};
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub fn router(services: ConversationServices) -> Router {
    Router::new()
        .route("/v1/conversations", post(start).get(list))
        .route("/v1/conversations/{conversation_id}/messages", post(send))
        .route(
            "/v1/conversations/{conversation_id}",
            axum::routing::delete(end),
        )
        .route("/v1/conversations/{conversation_id}/history", get(history))
        .with_state(services)
}

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body {
            error: String,
        }
        (self.0, Json(Body { error: self.1 })).into_response()
    }
}

fn anon_actor() -> EntityId<Actor> {
    EntityId::<Actor>::from_uuid(uuid::Uuid::nil())
}

fn role_str(r: Role) -> &'static str {
    match r {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct StartBody {
    user_id: String,
    title: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct StartResponse {
    conversation_id: String,
}

async fn start(
    State(s): State<ConversationServices>,
    Json(body): Json<StartBody>,
) -> Result<Json<StartResponse>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&body.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "malformed user_id".into()))?;
    let out = s
        .start
        .execute(StartConversationInput {
            user_id,
            title: body.title,
            actor_id: anon_actor(),
        })
        .await
        .map_err(|e| match e {
            StartConversationError::Repository(_) => {
                ApiError(StatusCode::SERVICE_UNAVAILABLE, "storage".into())
            }
            StartConversationError::Audit(_) => {
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, "audit".into())
            }
        })?;
    Ok(Json(StartResponse {
        conversation_id: out.conversation_id.to_string(),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct SendBody {
    body: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct SendResponse {
    reply: String,
    tone: String,
}

async fn send(
    State(s): State<ConversationServices>,
    Path(conversation_id): Path<String>,
    Json(body): Json<SendBody>,
) -> Result<Json<SendResponse>, ApiError> {
    let id = EntityId::<Conversation>::from_str(&conversation_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "malformed conversation_id".into()))?;
    let out = s
        .send
        .execute(SendMessageInput {
            conversation_id: id,
            body: body.body,
            actor_id: anon_actor(),
        })
        .await
        .map_err(|e| match e {
            SendMessageError::NotFound => ApiError(StatusCode::NOT_FOUND, "not found".into()),
            SendMessageError::Domain(d) => ApiError(StatusCode::BAD_REQUEST, d.to_string()),
            SendMessageError::Repository(_) => {
                ApiError(StatusCode::SERVICE_UNAVAILABLE, "storage".into())
            }
            SendMessageError::Llm(_) => ApiError(StatusCode::SERVICE_UNAVAILABLE, "llm".into()),
            SendMessageError::Audit(_) => {
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, "audit".into())
            }
        })?;
    Ok(Json(SendResponse {
        reply: out.reply,
        tone: out.tone.as_str().into(),
    }))
}

async fn end(
    State(s): State<ConversationServices>,
    Path(conversation_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = EntityId::<Conversation>::from_str(&conversation_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "malformed conversation_id".into()))?;
    s.end
        .execute(EndConversationInput {
            conversation_id: id,
            actor_id: anon_actor(),
        })
        .await
        .map_err(|e| match e {
            EndConversationError::NotFound => ApiError(StatusCode::NOT_FOUND, "not found".into()),
            EndConversationError::Repository(_) => {
                ApiError(StatusCode::SERVICE_UNAVAILABLE, "storage".into())
            }
            EndConversationError::Audit(_) => {
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, "audit".into())
            }
        })?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct MessageJson {
    id: String,
    role: String,
    body: String,
    tone: String,
    sent_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct HistoryResponse {
    messages: Vec<MessageJson>,
}

async fn history(
    State(s): State<ConversationServices>,
    Path(conversation_id): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, ApiError> {
    let id = EntityId::<Conversation>::from_str(&conversation_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "malformed conversation_id".into()))?;
    let out = s
        .history
        .execute(GetHistoryInput {
            conversation_id: id,
            limit: q.limit.unwrap_or(50),
        })
        .await
        .map_err(|e| match e {
            GetHistoryError::Repository(_) => {
                ApiError(StatusCode::SERVICE_UNAVAILABLE, "storage".into())
            }
        })?;
    Ok(Json(HistoryResponse {
        messages: out.messages.iter().map(as_json).collect(),
    }))
}

fn as_json(m: &Message) -> MessageJson {
    MessageJson {
        id: m.id().to_string(),
        role: role_str(m.role()).into(),
        body: m.body().to_string(),
        tone: m.tone().as_str().into(),
        sent_at: m.sent_at().to_rfc3339(),
    }
}

#[derive(Deserialize)]
struct ListQuery {
    user_id: String,
    limit: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ConversationJson {
    id: String,
    user_id: String,
    title: Option<String>,
    status: String,
    message_count: u32,
    started_at: String,
    ended_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ListResponse {
    conversations: Vec<ConversationJson>,
}

async fn list(
    State(s): State<ConversationServices>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    let user_id = EntityId::<UserRef>::from_str(&q.user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "malformed user_id".into()))?;
    let out = s
        .list
        .execute(ListConversationsInput {
            user_id,
            limit: q.limit.unwrap_or(20),
        })
        .await
        .map_err(|e| match e {
            ListConversationsError::Repository(_) => {
                ApiError(StatusCode::SERVICE_UNAVAILABLE, "storage".into())
            }
        })?;
    Ok(Json(ListResponse {
        conversations: out
            .conversations
            .iter()
            .map(|c| ConversationJson {
                id: c.id().to_string(),
                user_id: c.user_id().to_string(),
                title: c.title().map(std::string::ToString::to_string),
                status: match c.status() {
                    conversation_domain::ConversationStatus::Active => "active".into(),
                    conversation_domain::ConversationStatus::Ended => "ended".into(),
                },
                message_count: c.message_count(),
                started_at: c.started_at().to_rfc3339(),
                ended_at: c.ended_at().map(|t| t.to_rfc3339()),
            })
            .collect(),
    }))
}
