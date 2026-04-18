# ADR 0002 — Cloud Run for Service Hosting

**Status:** Accepted
**Date:** 2026-04-18
**Deciders:** Allan Smeyatsky

## Context

`Architectural Rules — 2026.md` §1 mandates GCP with Cloud Run, Secret Manager, IAM, and Workload Identity. The current deployment topology is Kubernetes (`k8s/`), with 33 security findings including base64 "secrets," wildcard CORS, and pods running as root (AUDIT §2). Self-hosted K8s is not the default and carries ongoing operational cost disproportionate to product stage.

## Decision

All services deploy to Cloud Run with Workload Identity binding service accounts to pod identities; all secrets live in Secret Manager and are referenced by name, never copied into config or env defaults. The existing `k8s/` manifests are retained only as reference during migration and are deleted once every service is live on Cloud Run.

## Consequences

- Secret Manager + Workload Identity replaces every current secret-handling path; this retires the CRITICAL audit findings around hardcoded credentials.
- Cloud Run's request-scoped concurrency model requires stateless handlers; any stateful infra (session store, rate-limit counters) moves to Redis or Firestore.
- ServiceMonitor / NetworkPolicy / Ingress issues in `k8s/` become moot rather than fixed.
- CI pipeline publishes container images to Artifact Registry and deploys with `gcloud run deploy`; no `kubectl` in the critical path.
