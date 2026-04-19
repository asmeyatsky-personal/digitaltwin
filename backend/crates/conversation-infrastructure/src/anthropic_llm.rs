//! Anthropic Messages API adapter for `LlmPort`. Timeouts, simple
//! circuit-breaker (§4). Per-AI-call attrs are recorded by the caller via the
//! `LlmResponse` fields (model, tokens_in/out) — §6.

use async_trait::async_trait;
use conversation_domain::{
    EmotionalTone, Message, Role,
    ports::{LlmError, LlmPort, LlmResponse},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

const DEFAULT_MODEL: &str = "claude-opus-4-7";
const SYSTEM_PROMPT: &str = "You are an emotional companion. Reply with two JSON fields in a single object: {\"body\": string, \"tone\": one of neutral|happy|sad|angry|anxious|surprised|calm|excited}. No other keys. No markdown fences.";

pub struct AnthropicLlm {
    client: Client,
    api_key: String,
    api_base: String,
    model: String,
    timeout: Duration,
    breaker: Breaker,
}

impl AnthropicLlm {
    #[must_use]
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self {
            client,
            api_key,
            api_base: "https://api.anthropic.com".into(),
            model: DEFAULT_MODEL.into(),
            timeout: Duration::from_secs(30),
            breaker: Breaker::new(5, Duration::from_secs(60)),
        }
    }

    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    #[must_use]
    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }
}

#[async_trait]
impl LlmPort for AnthropicLlm {
    async fn reply(
        &self,
        history: &[Message],
        user_message: &str,
    ) -> Result<LlmResponse, LlmError> {
        self.breaker.check()?;

        let messages: Vec<ApiMessage> = history
            .iter()
            .chain(std::iter::once(
                &Message::new(
                    conversation_domain::MessageId::new(),
                    Role::User,
                    user_message.to_string(),
                    EmotionalTone::Neutral,
                    chrono::Utc::now(),
                )
                .map_err(|e| LlmError::CallFailed(e.to_string()))?,
            ))
            .filter_map(|m| match m.role() {
                Role::System => None, // system prompt is sent separately
                Role::User => Some(ApiMessage {
                    role: "user".into(),
                    content: m.body().to_string(),
                }),
                Role::Assistant => Some(ApiMessage {
                    role: "assistant".into(),
                    content: m.body().to_string(),
                }),
            })
            .collect();

        let body = ApiRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            system: SYSTEM_PROMPT.into(),
            messages,
        };

        let url = format!("{}/v1/messages", self.api_base);
        let resp = tokio::time::timeout(
            self.timeout,
            self.client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .json(&body)
                .send(),
        )
        .await
        .map_err(|_| {
            self.breaker.record_failure();
            LlmError::Timeout
        })?
        .map_err(|e| {
            self.breaker.record_failure();
            LlmError::CallFailed(e.to_string())
        })?;

        if !resp.status().is_success() {
            self.breaker.record_failure();
            return Err(LlmError::CallFailed(format!("http {}", resp.status())));
        }

        let parsed: ApiResponse = resp.json().await.map_err(|e| {
            self.breaker.record_failure();
            LlmError::CallFailed(e.to_string())
        })?;

        let text = parsed
            .content
            .into_iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text),
                ContentBlock::Other => None,
            })
            .collect::<Vec<_>>()
            .join("");

        // Validate AI output against the explicit schema (§4). `text` must be a
        // JSON object with `body` and `tone`; reject anything else.
        let shape: LlmShape = serde_json::from_str(text.trim())
            .map_err(|e| LlmError::CallFailed(format!("non-JSON reply: {e}")))?;
        let tone = EmotionalTone::parse(&shape.tone)?;

        self.breaker.record_success();

        Ok(LlmResponse {
            body: shape.body,
            tone,
            model: parsed.model,
            tokens_in: parsed.usage.input_tokens,
            tokens_out: parsed.usage.output_tokens,
        })
    }
}

// ---- Anthropic wire types ----

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ApiMessage>,
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ApiResponse {
    model: String,
    content: Vec<ContentBlock>,
    usage: Usage,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct LlmShape {
    body: String,
    tone: String,
}

// ---- Tiny circuit breaker ----

struct Breaker {
    failure_threshold: u32,
    open_for: Duration,
    failure_count: AtomicU64,
    opened_at_millis: AtomicU64,
}

impl Breaker {
    fn new(failure_threshold: u32, open_for: Duration) -> Self {
        Self {
            failure_threshold,
            open_for,
            failure_count: AtomicU64::new(0),
            opened_at_millis: AtomicU64::new(0),
        }
    }

    fn check(&self) -> Result<(), LlmError> {
        let opened = self.opened_at_millis.load(Ordering::Relaxed);
        if opened == 0 {
            return Ok(());
        }
        let now = Self::now_millis();
        if now.saturating_sub(opened) > u64::try_from(self.open_for.as_millis()).unwrap_or(u64::MAX)
        {
            // Half-open: reset and try again.
            self.opened_at_millis.store(0, Ordering::Relaxed);
            self.failure_count.store(0, Ordering::Relaxed);
            Ok(())
        } else {
            Err(LlmError::CallFailed("circuit breaker open".into()))
        }
    }

    fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
    }

    fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        if u32::try_from(failures).unwrap_or(u32::MAX) >= self.failure_threshold {
            self.opened_at_millis
                .store(Self::now_millis(), Ordering::Relaxed);
        }
    }

    fn now_millis() -> u64 {
        let epoch = std::time::SystemTime::UNIX_EPOCH;
        std::time::SystemTime::now()
            .duration_since(epoch)
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0)
    }
}
// silence unused-import warnings if Instant goes away in future refactors
const _: Option<Instant> = None;
