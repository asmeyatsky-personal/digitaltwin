//! Layer: shared contracts (generated).
//! Ports: none — pure types.
//! MCP integration: MCP tool/resource schemas are derived from these messages.
//! Stack choice: canonical (Protobuf per §1).
//!
//! Generated Rust bindings for `contracts/identity/v1/identity.proto`.
//! Re-exported under `v1` so consumers write `identity_contracts::v1::*`.

#![allow(clippy::all, clippy::pedantic)]

pub mod v1 {
    tonic::include_proto!("digitaltwin.identity.v1");
}
