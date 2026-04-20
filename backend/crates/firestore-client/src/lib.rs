//! Layer: infrastructure (shared).
//! Ports: none — this crate exposes concrete types that other contexts'
//! adapters wrap. Each bounded context keeps its own typed `DocumentStore`
//! trait at the domain level and implements it with this client.
//! MCP integration: none directly.
//! Stack choice: canonical (Rust; Firestore per ADR-0003 for documents).
//!
//! Thin Firestore REST client. Auth is pluggable: `TokenSource::metadata()`
//! fetches access tokens from the GCE metadata server (Workload Identity on
//! Cloud Run); `TokenSource::service_account_json()` signs a JWT with a
//! service-account key for local development.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

mod auth;
mod client;
mod error;

pub use auth::{ServiceAccountKey, TokenSource};
pub use client::{Document, FirestoreClient, FirestoreValue};
pub use error::FirestoreError;
