//! Layer: presentation (Identity bounded context).
//! Ports: none — this crate only consumes the application layer.
//! MCP integration: `mcp` module hosts the Identity MCP server
//! (tools = writes, resources = reads per §3.5).
//! Stack choice: canonical.
//!
//! Thin gRPC + MCP adapters. Parses requests, delegates to use cases,
//! maps errors to transport-specific responses. Zero business logic.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

pub mod grpc;
pub mod mcp;
pub mod rest;

use std::sync::Arc;

use identity_application::{Authenticate, GetUser, RefreshToken, RegisterUser, RevokeToken};

/// Application services bundle. Passed into both the gRPC server and the MCP
/// server so they share a single composition root.
#[derive(Clone)]
pub struct IdentityServices {
    pub register_user: Arc<RegisterUser>,
    pub authenticate: Arc<Authenticate>,
    pub refresh_token: Arc<RefreshToken>,
    pub revoke_token: Arc<RevokeToken>,
    pub get_user: Arc<GetUser>,
}
