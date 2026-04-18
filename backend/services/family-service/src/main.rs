//! Composition root for the family bounded context. Cloud Run entrypoint.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = telemetry::init(telemetry::Config {
        service_name: "family-service".into(),
        otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
        log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()),
    })?;

    tracing::info!(
        "family-service scaffold up; adapters + MCP server land during legacy .NET port"
    );
    Ok(())
}
