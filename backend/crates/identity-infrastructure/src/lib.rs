//! Layer: infrastructure (Identity bounded context).
//! Ports: implements `identity_domain::ports::{UserRepository, PasswordHasher}`
//! and `identity_application::ports::TokenIssuer`.
//! MCP integration: none here; MCP wiring lives in `identity-presentation`.
//! Stack choice: canonical. Adapters pick concrete tech (Postgres, Argon2id,
//! RS256 JWT) but domain contracts do not leak the choice.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

pub mod argon2_hasher;
pub mod in_memory;
pub mod jwt_issuer;
pub mod postgres_audit_ledger;
pub mod postgres_token_blacklist;
pub mod postgres_user_repository;

pub use argon2_hasher::Argon2idHasher;
pub use jwt_issuer::{Rs256TokenIssuer, Rs256TokenIssuerError};
pub use postgres_audit_ledger::PostgresAuditLedger;
pub use postgres_token_blacklist::PostgresTokenBlacklist;
pub use postgres_user_repository::PostgresUserRepository;
