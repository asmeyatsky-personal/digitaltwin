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
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use therapy_application::{
    ListScreeningsForUser, ListTherapists, RegisterTherapist, SubmitScreening,
};
use therapy_infrastructure::PostgresTherapyRepository;
use therapy_presentation::{
    TherapyServices,
    mcp::{JsonRpcRequest, JsonRpcResponse, TherapyMcp},
};

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
        .merge(Env::prefixed("THERAPY_"))
        .merge(Env::raw().only(&["PORT"]))
        .extract()
        .context("config")?;
    let _g = telemetry::init(telemetry::Config {
        service_name: "therapy-service".into(),
        otlp_endpoint: cfg.otel_exporter_otlp_endpoint.clone(),
        log_level: cfg.log_level.clone(),
    })?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.database_url)
        .await?;
    let repo = Arc::new(PostgresTherapyRepository::new(pool.clone()));
    let audit = Arc::new(PostgresAuditLedger { pool });
    let clock = Arc::new(SystemClock);
    let services = TherapyServices {
        register: Arc::new(RegisterTherapist::new(
            repo.clone(),
            audit.clone(),
            clock.clone(),
        )),
        list_t: Arc::new(ListTherapists::new(repo.clone())),
        submit: Arc::new(SubmitScreening::new(repo.clone(), audit, clock)),
        list_s: Arc::new(ListScreeningsForUser::new(repo)),
    };
    let mcp = Arc::new(TherapyMcp::new(services.clone()));
    let http = Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route("/mcp", post(handle_mcp))
        .with_state(mcp)
        .merge(therapy_presentation::router(services))
        .layer(tower_http::cors::CorsLayer::permissive());
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port));
    tracing::info!(%addr, "therapy-service listening");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, http).await?;
    Ok(())
}
async fn handle_mcp(
    State(mcp): State<Arc<TherapyMcp>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(mcp.handle(req).await)
}
