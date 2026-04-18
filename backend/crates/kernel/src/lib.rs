//! Layer: domain (shared primitives).
//! Ports: none.
//! MCP integration: none.
//! Stack choice: canonical (Rust per §1).
//!
//! Shared value objects and traits used across bounded contexts. No I/O, no
//! adapter concerns. Stays dependency-free so every domain crate can consume
//! it without pulling in a runtime.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::module_name_repetitions)]

pub mod clock;
pub mod id;
pub mod pii;

pub use clock::Clock;
pub use id::{EntityId, IdError};
pub use pii::PiiString;
