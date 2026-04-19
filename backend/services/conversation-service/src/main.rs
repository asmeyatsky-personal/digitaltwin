//! Composition root for the Conversation bounded context. Cloud Run entrypoint.
//! Wires every port to a concrete adapter; starts gRPC (+ gRPC-Web), JSON REST,
//! and MCP on dedicated ports.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{Json, Router, extract::State, routing::post};
use conversation_application::{
    EndConversation, GetHistory, ListConversations, SendMessage, StartConversation,
};
use conversation_infrastructure::{
    AnthropicLlm, PostgresConversationRepository, PostgresMessageStore,
};
use conversation_presentation::{
    ConversationServices,
    grpc::ConversationGrpc,
    mcp::{ConversationMcp, JsonRpcRequest, JsonRpcResponse},
};
use figment::{Figment, providers::Env};
use kernel::clock::SystemClock;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;

#[derive(Debug, Deserialize)]
struct Config {
    port: u16,
    database_url: String,
    anthropic_api_key: String,
    #[serde(default)]
    otel_exporter_otlp_endpoint: Option<String>,
    #[serde(default = "default_log_level")]
    log_level: String,
}

fn default_log_level() -> String {
    "info".into()
}

fn load_config() -> Result<Config> {
    Figment::new()
        .merge(Env::prefixed("CONVERSATION_"))
        .merge(Env::raw().only(&["PORT"]))
        .extract::<Config>()
        .context("failed to load config (required: CONVERSATION_DATABASE_URL, CONVERSATION_ANTHROPIC_API_KEY, PORT)")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = load_config()?;

    let _guard = telemetry::init(telemetry::Config {
        service_name: "conversation-service".into(),
        otlp_endpoint: cfg.otel_exporter_otlp_endpoint.clone(),
        log_level: cfg.log_level.clone(),
    })?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.database_url)
        .await
        .context("postgres connect")?;

    // Audit ledger shim pending cross-context extraction — for now every
    // context needs its own Postgres-backed audit writer. We re-use the
    // Identity context's `PostgresAuditLedger` module via the shared `audit`
    // crate's `AuditPort`, instantiating a Conversation-scoped adapter here.
    let audit = Arc::new(PostgresAuditLedger::new(pool.clone()));

    let repo = Arc::new(PostgresConversationRepository::new(pool.clone()));
    let store = Arc::new(PostgresMessageStore::new(pool.clone()));
    let llm = Arc::new(AnthropicLlm::new(cfg.anthropic_api_key.clone()));
    let clock = Arc::new(SystemClock);

    let services = ConversationServices {
        start: Arc::new(StartConversation::new(
            repo.clone(),
            audit.clone(),
            clock.clone(),
        )),
        send: Arc::new(SendMessage::new(
            repo.clone(),
            store.clone(),
            llm,
            audit.clone(),
            clock.clone(),
        )),
        end: Arc::new(EndConversation::new(
            repo.clone(),
            audit.clone(),
            clock.clone(),
        )),
        history: Arc::new(GetHistory::new(store.clone())),
        list: Arc::new(ListConversations::new(repo.clone())),
    };

    let mcp = Arc::new(ConversationMcp::new(services.clone()));
    let mcp_router = Router::new()
        .route("/mcp", post(handle_mcp))
        .with_state(mcp);
    let rest_router = conversation_presentation::rest::router(services.clone());
    let cors = tower_http::cors::CorsLayer::permissive();
    let http = Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .merge(mcp_router)
        .merge(rest_router)
        .layer(cors);

    let http_addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port));
    let grpc_addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port + 1));
    tracing::info!(%http_addr, %grpc_addr, "conversation-service listening");

    let http_task = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(http_addr).await?;
        axum::serve(listener, http).await?;
        Ok::<_, anyhow::Error>(())
    });

    let grpc_task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .accept_http1(true)
            .layer(tonic_web::GrpcWebLayer::new())
            .add_service(ConversationGrpc::new(services))
            .serve(grpc_addr)
            .await?;
        Ok::<_, anyhow::Error>(())
    });

    tokio::select! {
        r = http_task => r??,
        r = grpc_task => r??,
        _ = tokio::signal::ctrl_c() => tracing::info!("shutdown signal received"),
    }
    Ok(())
}

async fn handle_mcp(
    State(mcp): State<Arc<ConversationMcp>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(mcp.handle(req).await)
}

// ---- audit adapter (stand-alone so conversation-service doesn't pull in
// identity-infrastructure) ---------------------------------------------------

use async_trait::async_trait;
use audit::{AuditError, AuditEvent, AuditPort};
use sqlx::PgPool;

struct PostgresAuditLedger {
    pool: PgPool,
}
impl PostgresAuditLedger {
    fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl AuditPort for PostgresAuditLedger {
    async fn append(&self, event: AuditEvent) -> Result<(), AuditError> {
        sqlx::query(
            "INSERT INTO audit.events \
             (occurred_at, actor_id, action, entity_type, entity_id, before_hash, after_hash) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(event.occurred_at)
        .bind(event.actor_id.as_uuid())
        .bind(event.action)
        .bind(event.entity_type)
        .bind(event.entity_id)
        .bind(event.before_hash)
        .bind(event.after_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| AuditError::AppendFailed(e.to_string()))?;
        Ok(())
    }
}
