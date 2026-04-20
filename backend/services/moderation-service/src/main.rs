#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use async_trait::async_trait;
use audit::{AuditError, AuditEvent, AuditPort};
use axum::{Json, Router, extract::State, routing::post};
use figment::{Figment, providers::Env};
use kernel::clock::SystemClock;
use moderation_application::{ListPending, ReportContent, ReviewReport};
use moderation_infrastructure::PostgresReportRepository;
use moderation_presentation::{
    ModerationServices,
    mcp::{JsonRpcRequest, JsonRpcResponse, ModerationMcp},
};
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Debug, Deserialize)]
struct Config {
    port: u16,
    database_url: String,
    #[serde(default)] otel_exporter_otlp_endpoint: Option<String>,
    #[serde(default = "default_log_level")] log_level: String,
}
fn default_log_level() -> String { "info".into() }

struct PostgresAuditLedger { pool: PgPool }
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
    let cfg: Config = Figment::new().merge(Env::prefixed("MODERATION_")).merge(Env::raw().only(&["PORT"])).extract().context("config")?;
    let _g = telemetry::init(telemetry::Config {
        service_name: "moderation-service".into(),
        otlp_endpoint: cfg.otel_exporter_otlp_endpoint.clone(),
        log_level: cfg.log_level.clone(),
    })?;
    let pool = PgPoolOptions::new().max_connections(10).acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.database_url).await?;
    let repo = Arc::new(PostgresReportRepository::new(pool.clone()));
    let audit = Arc::new(PostgresAuditLedger { pool });
    let clock = Arc::new(SystemClock);
    let services = ModerationServices {
        report: Arc::new(ReportContent::new(repo.clone(), audit.clone(), clock.clone())),
        review: Arc::new(ReviewReport::new(repo.clone(), audit, clock)),
        pending: Arc::new(ListPending::new(repo)),
    };
    let mcp = Arc::new(ModerationMcp::new(services.clone()));
    let http = Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route("/mcp", post(handle_mcp)).with_state(mcp)
        .merge(moderation_presentation::router(services))
        .layer(tower_http::cors::CorsLayer::permissive());
    let addr = std::net::SocketAddr::from(([0,0,0,0], cfg.port));
    tracing::info!(%addr, "moderation-service listening");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, http).await?;
    Ok(())
}
async fn handle_mcp(State(mcp): State<Arc<ModerationMcp>>, Json(req): Json<JsonRpcRequest>) -> Json<JsonRpcResponse> {
    Json(mcp.handle(req).await)
}
