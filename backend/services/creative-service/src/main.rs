#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use async_trait::async_trait;
use audit::{AuditError, AuditEvent, AuditPort};
use axum::{Json, Router, extract::State, routing::post};
use creative_application::{CreateWork, GetWork, ListWorks, ShareWork};
use creative_infrastructure::PostgresCreativeRepository;
use creative_presentation::{
    CreativeServices,
    mcp::{CreativeMcp, JsonRpcRequest, JsonRpcResponse},
};
use figment::{Figment, providers::Env};
use kernel::clock::SystemClock;
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Debug, Deserialize)]
struct Config {
    port: u16,
    database_url: String,
    #[serde(default)]
    otel_exporter_otlp_endpoint: Option<String>,
    #[serde(default = "default_log_level")]
    log_level: String,
}
fn default_log_level() -> String {
    "info".into()
}

struct PostgresAuditLedger {
    pool: PgPool,
}
#[async_trait]
impl AuditPort for PostgresAuditLedger {
    async fn append(&self, e: AuditEvent) -> Result<(), AuditError> {
        sqlx::query("INSERT INTO audit.events (occurred_at, actor_id, action, entity_type, entity_id, before_hash, after_hash) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(e.occurred_at).bind(e.actor_id.as_uuid()).bind(e.action)
            .bind(e.entity_type).bind(e.entity_id).bind(e.before_hash).bind(e.after_hash)
            .execute(&self.pool).await.map_err(|err| AuditError::AppendFailed(err.to_string()))?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg: Config = Figment::new()
        .merge(Env::prefixed("CREATIVE_"))
        .merge(Env::raw().only(&["PORT"]))
        .extract()
        .context("config")?;
    let _g = telemetry::init(telemetry::Config {
        service_name: "creative-service".into(),
        otlp_endpoint: cfg.otel_exporter_otlp_endpoint.clone(),
        log_level: cfg.log_level.clone(),
    })?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.database_url)
        .await?;
    let repo = Arc::new(PostgresCreativeRepository::new(pool.clone()));
    let audit = Arc::new(PostgresAuditLedger { pool });
    let clock = Arc::new(SystemClock);
    let services = CreativeServices {
        create: Arc::new(CreateWork::new(repo.clone(), audit, clock)),
        share: Arc::new(ShareWork::new(repo.clone())),
        get: Arc::new(GetWork::new(repo.clone())),
        list: Arc::new(ListWorks::new(repo)),
    };
    let mcp = Arc::new(CreativeMcp::new(services.clone()));
    let http = Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route("/mcp", post(handle_mcp))
        .with_state(mcp)
        .merge(creative_presentation::router(services))
        .layer(tower_http::cors::CorsLayer::permissive());
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port));
    tracing::info!(%addr, "creative-service listening");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, http).await?;
    Ok(())
}
async fn handle_mcp(
    State(mcp): State<Arc<CreativeMcp>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(mcp.handle(req).await)
}
