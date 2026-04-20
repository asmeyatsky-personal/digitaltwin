# Migration to Architectural Rules 2026

Status tracker for the shift from the .NET/K8s stack (`src/`, `k8s/`) to the
canonical Rust/Cloud Run/Firestore+Postgres stack defined by
`Architectural Rules — 2026.md` and the three foundational ADRs in
`docs/adr/`.

## ADRs

| ID  | Subject                                   | Status |
|-----|-------------------------------------------|--------|
| 0001 | Rust for backend                         | Accepted |
| 0002 | Cloud Run hosting                        | Accepted |
| 0003 | Postgres primary + Firestore for documents | Accepted |

## Backend workspace

All new backend code lives in `backend/` as a Cargo workspace. Layer direction
(§2 domain ← application ← infrastructure ← presentation) is enforced three
ways:

1. **Per-crate `Cargo.toml` deps** — a domain crate cannot import what isn't in
   its dependency list, so a violation is a compile error.
2. **`backend/deny.toml` `bans` table** — cargo-deny blocks transitive leaks
   (e.g. domain pulling in `tokio` through a well-meaning helper).
3. **CI gate** — `.github/workflows/backend.yml` runs `cargo deny check` and
   `cargo clippy -D warnings`. The rule is real.

Shared crates:

| Crate       | Purpose                                                    |
|-------------|------------------------------------------------------------|
| `kernel`    | Shared domain primitives (EntityId, Clock, PiiString)     |
| `audit`     | Append-only ledger — `AuditPort`, Postgres adapter        |
| `telemetry` | OTel tracing + JSON logs + Prometheus RED metrics         |

## Bounded contexts

Each context owns five crates plus a service binary:

| Suffix           | Purpose                                             |
|------------------|-----------------------------------------------------|
| `-contracts`     | Protobuf-generated types (`contracts/{ctx}/v1/…`)   |
| `-domain`        | Aggregates, value objects, port traits — zero I/O   |
| `-application`   | Use cases, orchestration, auth-flow ports          |
| `-infrastructure`| Adapters: DB, crypto, HTTP, external SDKs          |
| `-presentation`  | gRPC handlers + MCP server (§3.5 "one per context") |

### Status

| Context         | Scaffold | Domain | App | Infra | Presentation | MCP server | Notes |
|-----------------|:--:|:--:|:--:|:--:|:--:|:--:|-------|
| identity        | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Argon2id, RS256 JWT, Postgres + audit; E2E tested |
| conversation    | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Anthropic LLM, AD-1 unified tone; Postgres |
| emotion         | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Multi-modal fusion (AD-1); weighted by modality |
| avatar          | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Python avatar-generation-service proxy |
| voice           | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Python voice-service proxy (clone + TTS) |
| memory          | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Firestore for memories + life events |
| family          | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Postgres |
| achievement     | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Postgres |
| community       | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Postgres (posts could move to Firestore later) |
| moderation      | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Postgres |
| therapy         | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | PHQ-9/GAD-7 scoring with APA cut-points |
| learning        | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Postgres |
| creative        | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Postgres |
| notification    | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Postgres + Expo Push HTTP adapter |

✅ = done · ⬜ = to do

### Porting a context

Use the Identity context as the reference; repeat for each remaining context:

1. **Contracts first.** Flesh out `contracts/{ctx}/v1/{ctx}.proto` with
   request/response messages and the service RPCs. Writes become MCP tools;
   reads become MCP resources.
2. **Domain.** Aggregates as immutable records (`#[derive(Clone, Serialize)]`,
   state transitions return `Self`). Invariants in constructors/factories,
   ports as `async_trait` traits.
3. **Application.** Use cases take port trait objects via `Arc<dyn …>`. Every
   write calls `audit::AuditPort::append`.
4. **Infrastructure.** Postgres repos via `sqlx`, Firestore via REST, external
   SDKs with explicit timeouts + circuit breakers.
5. **Presentation.** gRPC handler (from `-contracts` generated server trait)
   + MCP server module (JSON-RPC 2.0, tools for writes, resources for reads).
6. **Service binary.** Composition root wiring telemetry + all adapters.
7. **Tests.** In-memory adapters (§3.2), coverage floors — 95% domain / 85%
   app / 80% overall — gated by `scripts/check-coverage.py`.
8. **Cloud Run.** Copy `deploy/cloud-run/identity-service.yaml`, substitute
   names, create Secret Manager secrets, bind the service account.
9. **Retire .NET.** Delete the corresponding controllers/services/entities
   from `src/` and the deployment from `k8s/` in the same PR.

## Python services

`services/*` stay in Python per §1 (AI/ML, orchestration). Required cleanups:

- Pin interpreter to 3.12+ in each `Dockerfile` and `pyproject.toml`
- Add `import-linter` config enforcing the same layer direction
- Add auth middleware + rate limiting (AUDIT §2.2 findings)
- Emit OTel traces to the same collector as the Rust services

## Frontends

`web/` (Next.js) and `mobile/` (Expo) stay per §1. `buf.gen.yaml` generates TS
bindings into `web/lib/contracts/` and `mobile/lib/contracts/`; replace any
hand-written DTOs with the generated types to kill the duplicate-DTO class of
bugs flagged in AUDIT §1.2.

## Retiring legacy artefacts

- `k8s/` — **deleted** (ADR-0002; Cloud Run is the canonical target)
- `src/` (.NET) — **deleted**; all 14 contexts live in `backend/` now
- `DigitalTwin.sln` — **deleted**
- `docker-compose.yml` — keep for local dev only; production is Cloud Run
- Elasticsearch references — still present in docker-compose.yml; remove
  when local-dev stops needing it
- `Assets/` (Unity building-management code) — pre-dates the emotional
  companion scope; retained only as reference and not built by CI

## Features not yet ported

The retired `.NET src/` had a handful of features without a 1:1 Rust context:
Biometric (HealthKit/Google Fit), Coaching (goal tracking), Insights
(aggregate analytics), Subscription (billing), CheckIn (proactive prompts).
Each maps to a future Rust bounded context using the same scaffold pattern
(`scripts/scaffold_context.py`).
