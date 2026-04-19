//! Conversation MCP server. Tools = writes (start, send, end); Resources =
//! reads (history). Same JSON-RPC 2.0 shape as the Identity MCP server.

use crate::ConversationServices;
use audit::Actor;
use conversation_application::{
    EndConversationInput, GetHistoryInput, SendMessageInput, StartConversationInput,
};
use conversation_domain::{Conversation, conversation::UserRef};
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::str::FromStr;
use tracing::instrument;

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
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }
    fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

pub struct ConversationMcp {
    services: ConversationServices,
}

impl ConversationMcp {
    #[must_use]
    pub fn new(services: ConversationServices) -> Self {
        Self { services }
    }

    #[instrument(skip(self, req))]
    pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone();
        if req.jsonrpc != "2.0" {
            return JsonRpcResponse::err(id, -32600, "jsonrpc must be 2.0");
        }
        match req.method.as_str() {
            "initialize" => JsonRpcResponse::ok(id, initialize_result()),
            "tools/list" => JsonRpcResponse::ok(id, tools_list()),
            "resources/list" => JsonRpcResponse::ok(id, resources_list()),
            "tools/call" => self.tool_call(id, req.params).await,
            "resources/read" => self.resource_read(id, req.params).await,
            other => JsonRpcResponse::err(id, -32601, format!("method not found: {other}")),
        }
    }

    async fn tool_call(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let args = params.get("arguments").cloned().unwrap_or(Value::Null);
        match name {
            "conversation.start" => {
                let Some(user_id_s) = args.get("user_id").and_then(Value::as_str) else {
                    return JsonRpcResponse::err(id, -32602, "user_id required");
                };
                let Ok(user_id) = EntityId::<UserRef>::from_str(user_id_s) else {
                    return JsonRpcResponse::err(id, -32602, "malformed user_id");
                };
                let title = args
                    .get("title")
                    .and_then(Value::as_str)
                    .map(std::string::ToString::to_string);
                match self
                    .services
                    .start
                    .execute(StartConversationInput {
                        user_id,
                        title,
                        actor_id: anon_actor(),
                    })
                    .await
                {
                    Ok(out) => JsonRpcResponse::ok(
                        id,
                        json!({ "content": [{ "type": "text", "text": out.conversation_id.to_string() }] }),
                    ),
                    Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                }
            }
            "conversation.send" => {
                let Some(conv_id_s) = args.get("conversation_id").and_then(Value::as_str) else {
                    return JsonRpcResponse::err(id, -32602, "conversation_id required");
                };
                let Ok(conversation_id) = EntityId::<Conversation>::from_str(conv_id_s) else {
                    return JsonRpcResponse::err(id, -32602, "malformed conversation_id");
                };
                let body = args.get("body").and_then(Value::as_str).unwrap_or_default();
                match self
                    .services
                    .send
                    .execute(SendMessageInput {
                        conversation_id,
                        body: body.into(),
                        actor_id: anon_actor(),
                    })
                    .await
                {
                    Ok(out) => JsonRpcResponse::ok(
                        id,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": out.reply,
                            }],
                            "tone": out.tone.as_str(),
                        }),
                    ),
                    Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                }
            }
            "conversation.end" => {
                let Some(conv_id_s) = args.get("conversation_id").and_then(Value::as_str) else {
                    return JsonRpcResponse::err(id, -32602, "conversation_id required");
                };
                let Ok(conversation_id) = EntityId::<Conversation>::from_str(conv_id_s) else {
                    return JsonRpcResponse::err(id, -32602, "malformed conversation_id");
                };
                match self
                    .services
                    .end
                    .execute(EndConversationInput {
                        conversation_id,
                        actor_id: anon_actor(),
                    })
                    .await
                {
                    Ok(()) => JsonRpcResponse::ok(
                        id,
                        json!({ "content": [{ "type": "text", "text": "ended" }] }),
                    ),
                    Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                }
            }
            other => JsonRpcResponse::err(id, -32602, format!("unknown tool: {other}")),
        }
    }

    async fn resource_read(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let uri = params
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(rest) = uri.strip_prefix("conversation://") else {
            return JsonRpcResponse::err(id, -32602, "unknown resource");
        };
        let Some((conv, _)) = rest.split_once("/history") else {
            return JsonRpcResponse::err(id, -32602, "only /history resource supported");
        };
        let Ok(conversation_id) = EntityId::<Conversation>::from_str(conv) else {
            return JsonRpcResponse::err(id, -32602, "malformed conversation id");
        };
        match self
            .services
            .history
            .execute(GetHistoryInput {
                conversation_id,
                limit: 50,
            })
            .await
        {
            Ok(out) => {
                let transcript: String = out
                    .messages
                    .iter()
                    .map(|m| {
                        format!(
                            "[{}] {}: {}",
                            m.tone().as_str(),
                            role_str(m.role()),
                            m.body()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                JsonRpcResponse::ok(
                    id,
                    json!({
                        "contents": [{
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": transcript,
                        }],
                    }),
                )
            }
            Err(e) => JsonRpcResponse::err(id, -32000, format!("{e:?}")),
        }
    }
}

fn anon_actor() -> EntityId<Actor> {
    EntityId::<Actor>::from_uuid(uuid::Uuid::nil())
}

fn role_str(r: conversation_domain::Role) -> &'static str {
    match r {
        conversation_domain::Role::User => "user",
        conversation_domain::Role::Assistant => "assistant",
        conversation_domain::Role::System => "system",
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "serverInfo": { "name": "digitaltwin-conversation", "version": env!("CARGO_PKG_VERSION") },
        "capabilities": { "tools": {}, "resources": {} },
    })
}

fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "conversation.start",
                "description": "Start a new conversation for a user (WRITE).",
                "inputSchema": {
                    "type": "object",
                    "required": ["user_id"],
                    "properties": {
                        "user_id": { "type": "string" },
                        "title":   { "type": "string" }
                    }
                }
            },
            {
                "name": "conversation.send",
                "description": "Send a user message and get the assistant reply (WRITE).",
                "inputSchema": {
                    "type": "object",
                    "required": ["conversation_id", "body"],
                    "properties": {
                        "conversation_id": { "type": "string" },
                        "body":            { "type": "string", "minLength": 1 }
                    }
                }
            },
            {
                "name": "conversation.end",
                "description": "End the conversation; further messages are rejected (WRITE).",
                "inputSchema": {
                    "type": "object",
                    "required": ["conversation_id"],
                    "properties": { "conversation_id": { "type": "string" } }
                }
            }
        ]
    })
}

fn resources_list() -> Value {
    json!({
        "resources": [
            {
                "uri": "conversation://{conversation_id}/history",
                "name": "conversation_history",
                "description": "Read the transcript of a conversation (READ).",
                "mimeType": "text/plain"
            }
        ]
    })
}
