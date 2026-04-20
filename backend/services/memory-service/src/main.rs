//! Composition root for the Memory bounded context.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use async_trait::async_trait;
use audit::{AuditError, AuditEvent, AuditPort};
use axum::{Json, Router, extract::State, routing::post};
use figment::{Figment, providers::Env};
use firestore_client::{FirestoreClient, ServiceAccountKey, TokenSource};
use kernel::clock::SystemClock;
use memory_application::{
    AddLifeEvent, GetConversationContext, GetTimeline, GetUpcoming, RecordMemory,
};
use memory_infrastructure::{FirestoreLifeEventStore, FirestoreMemoryStore};
use memory_presentation::{
    MemoryServices,
    mcp::{JsonRpcRequest, JsonRpcResponse, MemoryMcp},
};
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Debug, Deserialize)]
struct Config {
    port: u16,
    database_url: String,
    gcp_project_id: String,
    #[serde(default)]
    google_application_credentials_json: Option<String>,
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
        .merge(Env::prefixed("MEMORY_"))
        .merge(Env::raw().only(&["PORT"]))
        .extract::<Config>()
        .context("config (required: MEMORY_DATABASE_URL, MEMORY_GCP_PROJECT_ID, PORT)")
}

struct PostgresAuditLedger {
    pool: PgPool,
}
#[async_trait]
impl AuditPort for PostgresAuditLedger {
    async fn append(&self, e: AuditEvent) -> Result<(), AuditError> {
        sqlx::query(
            "INSERT INTO audit.events (occurred_at, actor_id, action, entity_type, entity_id, before_hash, after_hash) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(e.occurred_at).bind(e.actor_id.as_uuid()).bind(e.action)
        .bind(e.entity_type).bind(e.entity_id).bind(e.before_hash).bind(e.after_hash)
        .execute(&self.pool).await.map_err(|err| AuditError::AppendFailed(err.to_string()))?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = load_config()?;
    let _guard = telemetry::init(telemetry::Config {
        service_name: "memory-service".into(),
        otlp_endpoint: cfg.otel_exporter_otlp_endpoint.clone(),
        log_level: cfg.log_level.clone(),
    })?;

    // Audit ledger in Postgres; memories + life events in Firestore.
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.database_url)
        .await
        .context("postgres connect")?;

    // Firestore auth: service-account JSON locally, metadata server on Cloud Run.
    let tokens = match cfg.google_application_credentials_json.as_ref() {
        Some(json) => {
            let key = ServiceAccountKey::from_json(json).context("bad SA JSON")?;
            TokenSource::service_account(key)
        }
        None => TokenSource::metadata(),
    };
    let fs = FirestoreClient::new(cfg.gcp_project_id.clone(), tokens);

    let memory_store = Arc::new(FirestoreMemoryStore::new(fs.clone()));
    let event_store = Arc::new(FirestoreLifeEventStore::new(fs));
    let audit = Arc::new(PostgresAuditLedger { pool });
    let clock = Arc::new(SystemClock);

    let services = MemoryServices {
        record: Arc::new(RecordMemory::new(
            memory_store.clone(),
            audit.clone(),
            clock.clone(),
        )),
        timeline: Arc::new(GetTimeline::new(memory_store.clone())),
        add_event: Arc::new(AddLifeEvent::new(event_store.clone(), audit, clock.clone())),
        upcoming: Arc::new(GetUpcoming::new(event_store.clone())),
        context: Arc::new(GetConversationContext::new(
            memory_store,
            event_store,
            clock,
        )),
    };

    let mcp = Arc::new(MemoryMcp::new(services.clone()));
    let http = Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route("/mcp", post(handle_mcp))
        .with_state(mcp)
        .merge(memory_presentation::router(services))
        .layer(tower_http::cors::CorsLayer::permissive());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port));
    tracing::info!(%addr, "memory-service listening");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, http).await?;
    Ok(())
}

async fn handle_mcp(
    State(mcp): State<Arc<MemoryMcp>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(mcp.handle(req).await)
}
