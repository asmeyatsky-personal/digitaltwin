# ADR 0003 — Postgres Primary, Firestore for Documents

**Status:** Accepted
**Date:** 2026-04-18
**Deciders:** Allan Smeyatsky

## Context

`Architectural Rules — 2026.md` §1 specifies Postgres as the primary store and Firestore for documents. The domain contains both strongly relational data (identity, family memberships, session tokens, achievements, audit ledger) and unstructured/document-shaped data (memories, journal entries, life events, conversation transcripts, creative works, therapy screening responses, coaching plans). Elasticsearch is deployed but unused (AUDIT §4) and must be removed.

## Decision

**Postgres** holds all data with relational invariants, referential integrity, or audit-trail requirements: users, credentials, sessions, family/household graph, achievements, device tokens, push subscriptions, community memberships, audit ledger (separate role + IAM per §4).

**Firestore** holds append-mostly document data with flexible schemas: memories, journal entries, life events, conversation transcripts, creative works, learning-path progress snapshots, therapy screening responses, LLM-generated coaching plans.

**Redis** is cache-only: session lookups, rate-limit counters, ephemeral agent state.

**BigQuery** receives analytics and AI-call telemetry via streaming inserts. Elasticsearch is removed.

## Consequences

- Entities migrate to one store each; no dual-write during migration except for a brief cutover window per context.
- Firestore documents are versioned with a monotonic `schema_version` field; readers tolerate older versions.
- Aggregate boundaries align with store choice: an aggregate that spans both stores is a design smell and triggers a bounded-context split.
- Search over Firestore documents uses Firestore's native composite indexes; if full-text search is later required, we revisit via a new ADR rather than reintroducing Elasticsearch.
