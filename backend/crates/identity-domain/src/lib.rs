//! Layer: domain (Identity bounded context).
//! Ports: declared in this crate; implemented by `identity-infrastructure`.
//! MCP integration: use cases exposed via the Identity MCP server
//! (§3.5 "one MCP server per bounded context"), which lives in
//! `identity-presentation` and wraps `identity-application`.
//! Stack choice: canonical.
//!
//! Pure domain: aggregates, value objects, port traits, domain errors.
//! No I/O, no async runtime, no framework. All state transitions return new
//! instances (§3.3 immutable domain models). Invariants live in constructors
//! and factories (§3.4).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::module_name_repetitions)]

pub mod errors;
pub mod ports;
pub mod user;
pub mod values;

pub use errors::DomainError;
pub use user::{User, UserStatus};
pub use values::{Email, PasswordHash};
