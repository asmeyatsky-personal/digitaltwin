//! Layer: infrastructure (cross-cutting).
//! Ports: none — this is wiring, not a domain abstraction.
//! MCP integration: trace context propagates through MCP JSON-RPC via the
//! `traceparent` field in request headers (tonic) or in the top-level
//! envelope (HTTP POST `/mcp`).
//! Stack choice: canonical.
//!
//! OpenTelemetry + structured JSON logs + RED metrics pipeline. Initialised
//! once per service binary; other crates emit via the `tracing` and `metrics`
//! macros. Per-AI-call span attributes (model, tokens, cost) are recorded by
//! the caller — this crate only owns the pipeline (§6).

#![forbid(unsafe_code)]
#![deny(clippy::all)]

use metrics_exporter_prometheus::PrometheusBuilder;
use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, trace::TracerProvider};
use thiserror::Error;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("telemetry init failed: {0}")]
    Init(String),
}

pub struct Config {
    pub service_name: String,
    pub otlp_endpoint: Option<String>,
    pub log_level: String,
}

/// Held for the lifetime of the process. Dropping the guard flushes spans.
pub struct Guard {
    _provider: Option<TracerProvider>,
}

/// Initialise tracing + metrics + optional OTLP span export.
///
/// # Errors
/// Returns `TelemetryError::Init` when the OTLP exporter or Prometheus
/// recorder cannot be built.
#[allow(clippy::needless_pass_by_value)] // one-shot startup; ergonomics over ref
pub fn init(cfg: Config) -> Result<Guard, TelemetryError> {
    // Structured JSON logs — one line per event, no PII (PiiString wrappers
    // redact at the type level). Filter level from env or config default.
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&cfg.log_level))
        .map_err(|e| TelemetryError::Init(e.to_string()))?;

    let fmt_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_target(true)
        .with_level(true)
        .with_thread_ids(false);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    let provider = if let Some(endpoint) = cfg.otlp_endpoint.as_deref() {
        let exporter = SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()
            .map_err(|e| TelemetryError::Init(e.to_string()))?;

        let resource = Resource::new(vec![KeyValue::new(
            "service.name",
            cfg.service_name.clone(),
        )]);

        let provider = TracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .build();

        let tracer = provider.tracer(cfg.service_name.clone());
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        registry
            .with(otel_layer)
            .try_init()
            .map_err(|e| TelemetryError::Init(e.to_string()))?;
        global::set_tracer_provider(provider.clone());
        Some(provider)
    } else {
        registry
            .try_init()
            .map_err(|e| TelemetryError::Init(e.to_string()))?;
        None
    };

    // RED metrics exposed on :9100 by default. Cloud Run sidecar-free setup:
    // Prometheus scrapes the service directly via private VPC.
    let prom_bind = std::env::var("METRICS_BIND").unwrap_or_else(|_| "0.0.0.0:9100".into());
    let addr: std::net::SocketAddr = prom_bind
        .parse()
        .map_err(|e: std::net::AddrParseError| TelemetryError::Init(e.to_string()))?;
    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .map_err(|e| TelemetryError::Init(e.to_string()))?;

    tracing::info!(service = %cfg.service_name, metrics_addr = %addr, "telemetry initialised");

    Ok(Guard {
        _provider: provider,
    })
}

impl Drop for Guard {
    fn drop(&mut self) {
        global::shutdown_tracer_provider();
    }
}
