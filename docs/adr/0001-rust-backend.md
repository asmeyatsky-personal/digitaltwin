# ADR 0001 — Rust as Backend Language

**Status:** Accepted
**Date:** 2026-04-18
**Deciders:** Allan Smeyatsky

## Context

`Architectural Rules — 2026.md` §1 assigns Rust to ledgers, parsers, hot-path APIs (p99 < 50ms), and cryptography. Digital Twin's backend surface includes authentication/JWT issuance (cryptography), an append-only audit ledger (§4 audit-event rule), real-time emotion fusion and conversation orchestration that require sub-50ms tail latency, and inter-service MCP calls that must remain responsive under load. The existing implementation is C#/.NET 8 (`src/API`, `src/Core`, `src/Infrastructure`) and does not meet the stack default.

## Decision

All net-new backend code is written in Rust. Existing .NET code is migrated context-by-context and deleted once each Rust bounded context reaches functional parity. Python remains the stack default for AI/ML and agent orchestration (`services/*`); TypeScript + React remain the default for frontends (`web/`, `mobile/`).

## Consequences

- Eliminates C# from the canonical stack; no deviation ADR needed against §1.
- Migration is staged per bounded context (Identity → Conversation → EmotionFusion → …) to contain blast radius.
- Team must be proficient in Rust; tooling standardises on axum (HTTP), tonic (gRPC), sqlx (Postgres), Firestore REST, tracing, and metrics.
- During migration, two backends coexist; contracts are locked via Protobuf so clients are unaffected.
