//! Layer: service binary (composition root for the Identity bounded context).
//! Ports: wires every port in the Identity context to a concrete adapter.
//! MCP integration: starts the Identity MCP server on HTTP POST `/mcp`
//! alongside gRPC on `:PORT` (default 8080 per Cloud Run convention).
//! Stack choice: canonical.
//!
//! Cloud Run entrypoint. All runtime configuration comes from environment
//! variables bound to Secret Manager / Workload Identity (§4).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{Json, Router, extract::State, routing::post};
use chrono::Duration as ChronoDuration;
use figment::{Figment, providers::Env};
use identity_application::{Authenticate, GetUser, RefreshToken, RegisterUser, RevokeToken};
use identity_infrastructure::{
    Argon2idHasher, PostgresAuditLedger, PostgresTokenBlacklist, PostgresUserRepository,
    Rs256TokenIssuer,
};
use identity_presentation::{
    IdentityServices,
    grpc::IdentityGrpc,
    mcp::{IdentityMcp, JsonRpcRequest, JsonRpcResponse},
};
use kernel::clock::SystemClock;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;

#[derive(Debug, Deserialize)]
struct Config {
    port: u16,
    database_url: String,
    jwt_private_key_pem: String,
    jwt_public_key_pem: String,
    jwt_issuer: String,
    jwt_audience: String,
    #[serde(default = "default_access_ttl")]
    access_ttl_seconds: i64,
    #[serde(default = "default_refresh_ttl")]
    refresh_ttl_seconds: i64,
    #[serde(default)]
    otel_exporter_otlp_endpoint: Option<String>,
    #[serde(default = "default_log_level")]
    log_level: String,
}

fn default_access_ttl() -> i64 {
    900
} // 15 min
fn default_refresh_ttl() -> i64 {
    2_592_000
} // 30 days
fn default_log_level() -> String {
    "info".into()
}

fn load_config() -> Result<Config> {
    let raw = Figment::new()
        .merge(Env::prefixed("IDENTITY_"))
        .merge(Env::raw().only(&["PORT"]))
        .extract::<Config>();
    raw.context("failed to load config from environment (required: IDENTITY_DATABASE_URL, IDENTITY_JWT_PRIVATE_KEY_PEM, IDENTITY_JWT_PUBLIC_KEY_PEM, IDENTITY_JWT_ISSUER, IDENTITY_JWT_AUDIENCE, PORT)")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = load_config()?;

    let _guard = telemetry::init(telemetry::Config {
        service_name: "identity-service".into(),
        otlp_endpoint: cfg.otel_exporter_otlp_endpoint.clone(),
        log_level: cfg.log_level.clone(),
    })?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.database_url)
        .await
        .context("postgres connect")?;

    // Adapters.
    let users = Arc::new(PostgresUserRepository::new(pool.clone()));
    let hasher = Arc::new(Argon2idHasher::owasp_default());
    let tokens = Arc::new(
        Rs256TokenIssuer::new(
            cfg.jwt_private_key_pem.as_bytes(),
            cfg.jwt_public_key_pem.as_bytes(),
            cfg.jwt_issuer.clone(),
            cfg.jwt_audience.clone(),
            ChronoDuration::seconds(cfg.access_ttl_seconds),
            ChronoDuration::seconds(cfg.refresh_ttl_seconds),
        )
        .map_err(|e| anyhow::anyhow!("jwt issuer: {e}"))?,
    );
    let blacklist = Arc::new(PostgresTokenBlacklist::new(pool.clone()));
    let audit = Arc::new(PostgresAuditLedger::new(pool.clone()));
    let clock = Arc::new(SystemClock);

    // Use cases.
    let services = IdentityServices {
        register_user: Arc::new(RegisterUser::new(
            users.clone(),
            hasher.clone(),
            audit.clone(),
            clock.clone(),
        )),
        authenticate: Arc::new(Authenticate::new(
            users.clone(),
            hasher.clone(),
            tokens.clone(),
            clock.clone(),
        )),
        refresh_token: Arc::new(RefreshToken::new(
            users.clone(),
            tokens.clone(),
            blacklist.clone(),
            clock.clone(),
        )),
        revoke_token: Arc::new(RevokeToken::new(tokens.clone(), blacklist.clone())),
        get_user: Arc::new(GetUser::new(users.clone())),
    };

    // HTTP router: MCP + JSON REST + healthcheck. Web/mobile call the REST
    // endpoints; Claude-style agents call MCP; gRPC clients talk to the
    // tonic server on grpc_addr (below).
    let mcp = Arc::new(IdentityMcp::new(services.clone()));
    let mcp_router = Router::new()
        .route("/mcp", post(handle_mcp))
        .with_state(mcp);
    let rest_router = identity_presentation::rest::router(services.clone());
    let cors = tower_http::cors::CorsLayer::permissive();
    let http = Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .merge(mcp_router)
        .merge(rest_router)
        .layer(cors);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port));
    tracing::info!(%addr, "identity-service listening (gRPC + HTTP/MCP on same port)");

    // tonic::transport::Server and axum share a port via tonic's `into_router`
    // style; to keep dependencies sane we split ports: gRPC on `port`, HTTP on
    // `port + 1`. Cloud Run routes one port per service anyway.
    let http_addr = addr;
    let grpc_addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port + 1));

    let http_task = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(http_addr).await?;
        axum::serve(listener, http).await?;
        Ok::<_, anyhow::Error>(())
    });

    let grpc_task = tokio::spawn(async move {
        // tonic-web lets browsers hit the same service via gRPC-Web
        // (HTTP/1.1 + base64 framing). Mobile clients continue to use native
        // gRPC on the same listener.
        tonic::transport::Server::builder()
            .accept_http1(true)
            .layer(tonic_web::GrpcWebLayer::new())
            .add_service(IdentityGrpc::new(services))
            .serve(grpc_addr)
            .await?;
        Ok::<_, anyhow::Error>(())
    });

    tokio::select! {
        r = http_task => r??,
        r = grpc_task => r??,
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutdown signal received");
        }
    }
    Ok(())
}

async fn handle_mcp(
    State(mcp): State<Arc<IdentityMcp>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(mcp.handle(req).await)
}
