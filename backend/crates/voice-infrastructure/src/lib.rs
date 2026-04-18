//! Layer: infrastructure (voice bounded context).
//! Ports: defined in this context's domain or application crate; adapters
//! implement them in `-infrastructure`.
//! MCP integration: one MCP server per bounded context (§3.5) — lives in
//! `-presentation` for this context.
//! Stack choice: canonical (Rust backend per ADR-0001).
//!
//! Stub scaffold. Domain, use cases, adapters, and MCP tools land as the
//! feature ports from the legacy .NET service.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
