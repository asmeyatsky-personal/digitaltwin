//! Composition root for the Emotion bounded context. Cloud Run entrypoint.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use async_trait::async_trait;
use audit::{AuditError, AuditEvent, AuditPort};
use axum::{Json, Router, extract::State, routing::post};
use emotion_application::{FuseCurrent, GetTimeline, ReportReading};
use emotion_infrastructure::PostgresReadingRepository;
use emotion_presentation::{
    EmotionServices,
    grpc::EmotionGrpc,
    mcp::{EmotionMcp, JsonRpcRequest, JsonRpcResponse},
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

fn load_config() -> Result<Config> {
    Figment::new()
        .merge(Env::prefixed("EMOTION_"))
        .merge(Env::raw().only(&["PORT"]))
        .extract::<Config>()
        .context("config (required: EMOTION_DATABASE_URL, PORT)")
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
        .bind(e.occurred_at)
        .bind(e.actor_id.as_uuid())
        .bind(e.action)
        .bind(e.entity_type)
        .bind(e.entity_id)
        .bind(e.before_hash)
        .bind(e.after_hash)
        .execute(&self.pool)
        .await
        .map_err(|err| AuditError::AppendFailed(err.to_string()))?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = load_config()?;
    let _guard = telemetry::init(telemetry::Config {
        service_name: "emotion-service".into(),
        otlp_endpoint: cfg.otel_exporter_otlp_endpoint.clone(),
        log_level: cfg.log_level.clone(),
    })?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.database_url)
        .await
        .context("postgres connect")?;

    let repo = Arc::new(PostgresReadingRepository::new(pool.clone()));
    let audit = Arc::new(PostgresAuditLedger { pool: pool.clone() });
    let clock = Arc::new(SystemClock);

    let services = EmotionServices {
        report: Arc::new(ReportReading::new(repo.clone(), audit, clock.clone())),
        current: Arc::new(FuseCurrent::new(repo.clone(), clock)),
        timeline: Arc::new(GetTimeline::new(repo)),
    };

    let mcp = Arc::new(EmotionMcp::new(services.clone()));
    let mcp_router = Router::new()
        .route("/mcp", post(handle_mcp))
        .with_state(mcp);
    let rest_router = emotion_presentation::rest::router(services.clone());
    let http = Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .merge(mcp_router)
        .merge(rest_router)
        .layer(tower_http::cors::CorsLayer::permissive());

    let http_addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port));
    let grpc_addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port + 1));

    let http_task = tokio::spawn(async move {
        axum::serve(tokio::net::TcpListener::bind(http_addr).await?, http).await?;
        Ok::<_, anyhow::Error>(())
    });
    let grpc_task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .accept_http1(true)
            .layer(tonic_web::GrpcWebLayer::new())
            .add_service(EmotionGrpc::new(services))
            .serve(grpc_addr)
            .await?;
        Ok::<_, anyhow::Error>(())
    });

    tokio::select! {
        r = http_task => r??,
        r = grpc_task => r??,
        _ = tokio::signal::ctrl_c() => {},
    }
    Ok(())
}

async fn handle_mcp(
    State(mcp): State<Arc<EmotionMcp>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(mcp.handle(req).await)
}
