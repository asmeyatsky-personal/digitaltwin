#!/usr/bin/env python3
"""Scaffold a new bounded context in backend/ and contracts/.

Generates the four layer crates + service binary + proto skeleton that every
context in the Architectural Rules 2026 stack requires. The Identity context
is the reference implementation; this script replicates its shape for each
new context so the mass migration (task #10) is mechanical.

Usage:
    python3 scripts/scaffold_context.py <context_name> [<context_name> ...]
"""
from __future__ import annotations

import sys
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BACKEND = ROOT / "backend"
CONTRACTS = ROOT / "contracts"


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        print(f"skip (exists): {path.relative_to(ROOT)}")
        return
    path.write_text(content)
    print(f"wrote: {path.relative_to(ROOT)}")


def layer_comment(ctx: str, layer: str) -> str:
    return textwrap.dedent(f"""\
        //! Layer: {layer} ({ctx} bounded context).
        //! Ports: defined in this context's domain or application crate; adapters
        //! implement them in `-infrastructure`.
        //! MCP integration: one MCP server per bounded context (§3.5) — lives in
        //! `-presentation` for this context.
        //! Stack choice: canonical (Rust backend per ADR-0001).
        //!
        //! Stub scaffold. Domain, use cases, adapters, and MCP tools land as the
        //! feature ports from the legacy .NET service.

        #![forbid(unsafe_code)]
        #![deny(clippy::all, clippy::pedantic)]
        """)


def domain(ctx: str) -> None:
    crate = BACKEND / "crates" / f"{ctx}-domain"
    write(crate / "Cargo.toml", textwrap.dedent(f"""\
        [package]
        name = "{ctx}-domain"
        version = "0.1.0"
        edition.workspace = true
        rust-version.workspace = true
        license.workspace = true
        publish.workspace = true

        [dependencies]
        kernel = {{ workspace = true }}
        thiserror = {{ workspace = true }}
        serde = {{ workspace = true }}
        chrono = {{ workspace = true }}
        async-trait = {{ workspace = true }}
        """))
    write(crate / "src" / "lib.rs", layer_comment(ctx, "domain"))


def application(ctx: str) -> None:
    crate = BACKEND / "crates" / f"{ctx}-application"
    write(crate / "Cargo.toml", textwrap.dedent(f"""\
        [package]
        name = "{ctx}-application"
        version = "0.1.0"
        edition.workspace = true
        rust-version.workspace = true
        license.workspace = true
        publish.workspace = true

        [dependencies]
        kernel = {{ workspace = true }}
        {ctx}-domain = {{ workspace = true }}
        audit = {{ workspace = true }}
        async-trait = {{ workspace = true }}
        thiserror = {{ workspace = true }}
        serde = {{ workspace = true }}
        tracing = {{ workspace = true }}
        chrono = {{ workspace = true }}
        """))
    write(crate / "src" / "lib.rs", layer_comment(ctx, "application"))


def infrastructure(ctx: str) -> None:
    crate = BACKEND / "crates" / f"{ctx}-infrastructure"
    write(crate / "Cargo.toml", textwrap.dedent(f"""\
        [package]
        name = "{ctx}-infrastructure"
        version = "0.1.0"
        edition.workspace = true
        rust-version.workspace = true
        license.workspace = true
        publish.workspace = true

        [dependencies]
        kernel = {{ workspace = true }}
        audit = {{ workspace = true }}
        {ctx}-domain = {{ workspace = true }}
        {ctx}-application = {{ workspace = true }}
        async-trait = {{ workspace = true }}
        thiserror = {{ workspace = true }}
        tokio = {{ workspace = true }}
        sqlx = {{ workspace = true }}
        reqwest = {{ workspace = true }}
        tracing = {{ workspace = true }}
        serde = {{ workspace = true }}
        serde_json = {{ workspace = true }}
        chrono = {{ workspace = true }}
        uuid = {{ workspace = true }}
        """))
    write(crate / "src" / "lib.rs", layer_comment(ctx, "infrastructure"))


def contracts_crate(ctx: str) -> None:
    crate = BACKEND / "crates" / f"{ctx}-contracts"
    write(crate / "Cargo.toml", textwrap.dedent(f"""\
        [package]
        name = "{ctx}-contracts"
        version = "0.1.0"
        edition.workspace = true
        rust-version.workspace = true
        license.workspace = true
        publish.workspace = true

        [dependencies]
        prost = {{ workspace = true }}
        prost-types = "0.13"
        tonic = {{ workspace = true }}
        serde = {{ workspace = true }}

        [build-dependencies]
        tonic-build = "0.12"
        """))
    write(crate / "build.rs", textwrap.dedent(f"""\
        fn main() -> Result<(), Box<dyn std::error::Error>> {{
            let proto_root = std::path::Path::new("../../../contracts");
            let proto_file = proto_root.join("digitaltwin/{ctx}/v1/{ctx}.proto");
            println!("cargo:rerun-if-changed={{}}", proto_file.display());
            tonic_build::configure()
                .build_client(true)
                .build_server(true)
                .compile_protos(
                    &[proto_file.to_str().ok_or("bad path")?],
                    &[proto_root.to_str().ok_or("bad path")?],
                )?;
            Ok(())
        }}
        """))
    write(crate / "src" / "lib.rs", textwrap.dedent(f"""\
        //! Layer: shared contracts (generated) for the {ctx} bounded context.
        #![allow(clippy::all, clippy::pedantic)]
        pub mod v1 {{
            tonic::include_proto!("digitaltwin.{ctx}.v1");
        }}
        """))


def presentation(ctx: str) -> None:
    crate = BACKEND / "crates" / f"{ctx}-presentation"
    write(crate / "Cargo.toml", textwrap.dedent(f"""\
        [package]
        name = "{ctx}-presentation"
        version = "0.1.0"
        edition.workspace = true
        rust-version.workspace = true
        license.workspace = true
        publish.workspace = true

        [dependencies]
        kernel = {{ workspace = true }}
        audit = {{ workspace = true }}
        {ctx}-application = {{ workspace = true }}
        {ctx}-domain = {{ workspace = true }}
        {ctx}-contracts = {{ workspace = true }}
        async-trait = {{ workspace = true }}
        axum = {{ workspace = true }}
        tower = {{ workspace = true }}
        tower-http = {{ workspace = true }}
        http = {{ workspace = true }}
        serde = {{ workspace = true }}
        serde_json = {{ workspace = true }}
        thiserror = {{ workspace = true }}
        tracing = {{ workspace = true }}
        tokio = {{ workspace = true }}
        tonic = {{ workspace = true }}
        prost-types = "0.13"
        chrono = {{ workspace = true }}
        uuid = {{ workspace = true }}
        anyhow = {{ workspace = true }}
        """))
    write(crate / "src" / "lib.rs", layer_comment(ctx, "presentation"))


def service(ctx: str) -> None:
    crate = BACKEND / "services" / f"{ctx}-service"
    write(crate / "Cargo.toml", textwrap.dedent(f"""\
        [package]
        name = "{ctx}-service"
        version = "0.1.0"
        edition.workspace = true
        rust-version.workspace = true
        license.workspace = true
        publish.workspace = true

        [[bin]]
        name = "{ctx}-service"
        path = "src/main.rs"

        [dependencies]
        kernel = {{ workspace = true }}
        audit = {{ workspace = true }}
        telemetry = {{ workspace = true }}
        {ctx}-application = {{ workspace = true }}
        {ctx}-contracts = {{ workspace = true }}
        {ctx}-domain = {{ workspace = true }}
        {ctx}-infrastructure = {{ workspace = true }}
        {ctx}-presentation = {{ workspace = true }}
        tokio = {{ workspace = true }}
        tracing = {{ workspace = true }}
        anyhow = {{ workspace = true }}
        """))
    write(crate / "src" / "main.rs", textwrap.dedent(f"""\
        //! Composition root for the {ctx} bounded context. Cloud Run entrypoint.

        #![forbid(unsafe_code)]
        #![deny(clippy::all, clippy::pedantic)]

        #[tokio::main]
        async fn main() -> anyhow::Result<()> {{
            let _guard = telemetry::init(telemetry::Config {{
                service_name: "{ctx}-service".into(),
                otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
                log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()),
            }})?;

            tracing::info!("{ctx}-service scaffold up; adapters + MCP server land during legacy .NET port");
            Ok(())
        }}
        """))


def proto(ctx: str) -> None:
    path = CONTRACTS / "digitaltwin" / ctx / "v1" / f"{ctx}.proto"
    service_name = "".join(part.title() for part in ctx.split("_"))
    write(path, textwrap.dedent(f"""\
        syntax = "proto3";

        package digitaltwin.{ctx}.v1;

        // Skeleton contract for the {ctx} bounded context. Concrete RPCs land
        // during the legacy .NET port; this file exists so the context's Rust
        // contracts crate has something to compile.

        service {service_name}Service {{
          rpc Health(HealthRequest) returns (HealthResponse);
        }}

        message HealthRequest {{}}
        message HealthResponse {{
          string status = 1;
        }}
        """))


def scaffold(ctx: str) -> None:
    print(f"\n== {ctx} ==")
    proto(ctx)
    contracts_crate(ctx)
    domain(ctx)
    application(ctx)
    infrastructure(ctx)
    presentation(ctx)
    service(ctx)


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: scaffold_context.py <context> [<context> ...]", file=sys.stderr)
        return 2
    for ctx in sys.argv[1:]:
        scaffold(ctx)
    return 0


if __name__ == "__main__":
    sys.exit(main())
