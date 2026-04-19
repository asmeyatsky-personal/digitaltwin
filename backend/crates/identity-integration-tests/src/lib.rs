//! E2E wiring helpers shared across the integration tests in `tests/`.
//!
//! Spins up a throwaway Postgres via testcontainers, applies the Identity +
//! Audit migrations, and builds a fully-wired `IdentityServices` bundle with
//! real adapters (Argon2id, RS256 JWT, Postgres repositories, audit ledger).

#![allow(clippy::missing_errors_doc)]
