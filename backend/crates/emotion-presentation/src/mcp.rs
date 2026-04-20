//! Emotion MCP server.

use crate::EmotionServices;
use audit::Actor;
use emotion_application::ReportReadingInput;
use emotion_domain::{Modality, UnifiedTone, reading::UserRef};
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::str::FromStr;

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
    fn err(id: Option<Value>, code: i32, msg: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: msg.into(),
            }),
        }
    }
}

pub struct EmotionMcp {
    services: EmotionServices,
}
impl EmotionMcp {
    #[must_use]
    pub fn new(services: EmotionServices) -> Self {
        Self { services }
    }

    pub async fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone();
        if req.jsonrpc != "2.0" {
            return JsonRpcResponse::err(id, -32600, "jsonrpc must be 2.0");
        }
        match req.method.as_str() {
            "initialize" => JsonRpcResponse::ok(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": { "name": "digitaltwin-emotion", "version": env!("CARGO_PKG_VERSION") },
                    "capabilities": { "tools": {}, "resources": {} }
                }),
            ),
            "tools/list" => JsonRpcResponse::ok(
                id,
                json!({
                    "tools": [{
                        "name": "emotion.report_reading",
                        "description": "Report an emotion reading from face/voice/text/biometric (WRITE).",
                        "inputSchema": {
                            "type": "object",
                            "required": ["user_id", "modality", "tone", "confidence"],
                            "properties": {
                                "user_id": {"type": "string"},
                                "modality": {"type": "string", "enum": ["face", "voice", "text", "biometric"]},
                                "tone": {"type": "string", "enum": ["neutral","happy","sad","angry","anxious","surprised","calm","excited"]},
                                "confidence": {"type": "number", "minimum": 0, "maximum": 1}
                            }
                        }
                    }]
                }),
            ),
            "resources/list" => JsonRpcResponse::ok(
                id,
                json!({
                    "resources": [{
                        "uri": "emotion://{user_id}/current",
                        "name": "current_emotion",
                        "description": "Fused current emotional state over a 5-minute window (READ).",
                        "mimeType": "application/json"
                    }]
                }),
            ),
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
        if name != "emotion.report_reading" {
            return JsonRpcResponse::err(id, -32602, format!("unknown tool: {name}"));
        }
        let Some(user_id_s) = args.get("user_id").and_then(Value::as_str) else {
            return JsonRpcResponse::err(id, -32602, "user_id required");
        };
        let Ok(user_id) = EntityId::<UserRef>::from_str(user_id_s) else {
            return JsonRpcResponse::err(id, -32602, "bad user_id");
        };
        let Ok(modality) = Modality::parse(
            args.get("modality")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        ) else {
            return JsonRpcResponse::err(id, -32602, "bad modality");
        };
        let Ok(tone) =
            UnifiedTone::parse(args.get("tone").and_then(Value::as_str).unwrap_or_default())
        else {
            return JsonRpcResponse::err(id, -32602, "bad tone");
        };
        let confidence = args
            .get("confidence")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as f32;
        match self
            .services
            .report
            .execute(ReportReadingInput {
                user_id,
                modality,
                tone,
                confidence,
                actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
            })
            .await
        {
            Ok(out) => JsonRpcResponse::ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": out.reading_id.to_string() }]
                }),
            ),
            Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
        }
    }

    async fn resource_read(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let uri = params
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(rest) = uri.strip_prefix("emotion://") else {
            return JsonRpcResponse::err(id, -32602, "unknown resource");
        };
        let Some((user, _)) = rest.split_once("/current") else {
            return JsonRpcResponse::err(id, -32602, "only /current supported");
        };
        let Ok(user_id) = EntityId::<UserRef>::from_str(user) else {
            return JsonRpcResponse::err(id, -32602, "bad user id");
        };
        match self.services.current.execute(user_id).await {
            Ok(out) => JsonRpcResponse::ok(
                id,
                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string(&json!({
                            "tone": out.fused.tone.as_str(),
                            "confidence": out.fused.confidence,
                            "reading_count": out.fused.reading_count,
                            "window_start": out.fused.window_start.to_rfc3339(),
                            "window_end": out.fused.window_end.to_rfc3339(),
                        })).unwrap_or_default(),
                    }]
                }),
            ),
            Err(e) => JsonRpcResponse::err(id, -32000, format!("{e:?}")),
        }
    }
}
