# Digital Twin — Operational Runbook

## Service Overview

```
┌─────────────┐     ┌──────────────┐     ┌──────────────────┐
│   Nginx     │────▶│  .NET API    │────▶│  PostgreSQL 15   │
│  (reverse   │     │  Gateway     │     │  + pgvector      │
│   proxy)    │     │  :80/:443    │     │  :5432           │
└─────────────┘     └──────┬───────┘     └──────────────────┘
                           │
              ┌────────────┼────────────┬───────────┐
              ▼            ▼            ▼           ▼
        ┌──────────┐ ┌──────────┐ ┌─────────┐ ┌─────────┐
        │ DeepFace │ │   LLM    │ │  Voice  │ │ Avatar  │
        │  :8001   │ │  :8004   │ │  :8003  │ │  :8002  │
        └──────────┘ └──────────┘ └─────────┘ └─────────┘
```

**Supporting Infrastructure:**
| Service | Port | Purpose |
|---------|------|---------|
| Redis | 6379 | Cache + rate limiting |
| RabbitMQ | 5672 (15672 UI) | Event bus |
| Prometheus | 9090 | Metrics collection |
| Grafana | 3001 | Dashboards |
| Elasticsearch | 9200 | Log storage |
| Kibana | 5601 | Log UI |
| Jaeger | 16686 | Distributed tracing |
| MinIO | 9000 (9001 UI) | Object storage |

## Startup Procedure

```bash
# Full stack startup
docker compose up -d

# Verify health (wait ~60s for all services to initialize)
curl http://localhost:8080/health | jq .

# Expected: all checks show "healthy"
# DeepFace may take 60-90s on first startup (model loading)
```

## Health Endpoints

| Endpoint | Purpose | K8s Probe |
|----------|---------|-----------|
| `/health` | Full dependency check (DB, Redis, all microservices) | readinessProbe |
| `/health/live` | Lightweight liveness check (no dependency checks) | livenessProbe |

## Common Operations

### Restarting a single service
```bash
docker compose restart deepface        # Restart emotion service
docker compose restart llm-service     # Restart LLM service
docker compose restart api-gateway     # Restart .NET API
```

### Scaling (Kubernetes)
```bash
kubectl -n digitaltwin scale deployment digitaltwin-api --replicas=5
kubectl -n digitaltwin get hpa  # Check autoscaler status
```

### Viewing logs
```bash
# API gateway logs
docker compose logs -f api-gateway --tail=100

# All Python service logs
docker compose logs -f deepface llm-service voice-service avatar-generation

# Centralized: Kibana at http://localhost:5601
```

### Database operations
```bash
# Connect to PostgreSQL
docker compose exec postgres psql -U devuser -d digitaltwin

# Backup
docker compose exec postgres pg_dump -U devuser digitaltwin > backup_$(date +%Y%m%d).sql

# Restore
docker compose exec -T postgres psql -U devuser digitaltwin < backup_20260101.sql
```

## Monitoring

- **Grafana**: http://localhost:3001 (admin/admin)
  - Dashboard: "Digital Twin - Overview"
  - Key panels: Request Rate, Latency p95, Error Rate, WebSocket Connections
- **Prometheus**: http://localhost:9090
  - Active alerts: http://localhost:9090/alerts
- **Jaeger**: http://localhost:16686
  - Trace requests through the full pipeline
- **Kibana**: http://localhost:5601
  - Security events, error logs, audit trail

## Alert Reference

| Alert | Severity | Threshold | Action |
|-------|----------|-----------|--------|
| HighErrorRate | critical | >5% 5xx for 5min | Check API logs, verify DB connectivity |
| HighLatency | warning | p95 >2s for 5min | Check slow queries, microservice health |
| MicroserviceDown | critical | Service unreachable 1min | Restart service, check container logs |
| HighMemoryUsage | warning | >400MB for 10min | Check for memory leaks, restart if needed |
| PostgresDown | critical | DB unreachable | Check disk space, connection limits |
| RedisDown | critical | Cache unreachable | Restart Redis, check memory |

## Incident Response

### Circuit breaker is open
The Polly circuit breaker will open after 50% failure rate in a 30s window (min 5 requests). It auto-recovers after 30s (half-open state).

1. Check which service is failing: `curl localhost:8080/health | jq '.checks'`
2. Check service logs: `docker compose logs <service-name> --tail=50`
3. Restart if needed: `docker compose restart <service-name>`
4. The circuit breaker will automatically close once the service recovers

### Database connection exhaustion
1. Check active connections: `SELECT count(*) FROM pg_stat_activity;`
2. Kill idle connections: `SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state = 'idle' AND query_start < now() - interval '10 minutes';`
3. Check EF Core connection pool settings in docker-compose environment

### LLM provider outage
The LLM service supports multiple providers (OpenAI, Anthropic, Google). If one is down:
1. Check which provider is active: `curl localhost:8004/health | jq .`
2. Switch provider via environment variable in docker-compose
3. Restart: `docker compose restart llm-service`

## Environment Variables Reference

See `docker-compose.yml` lines 13-35 for the complete list. Key variables:

| Variable | Description |
|----------|-------------|
| `ConnectionStrings__DefaultConnection` | PostgreSQL connection string |
| `Redis__ConnectionString` | Redis connection string |
| `RabbitMQ__ConnectionString` | RabbitMQ connection string |
| `Services__DeepFace__BaseUrl` | DeepFace service URL |
| `Services__LLM__BaseUrl` | LLM service URL |
| `Services__Avatar__BaseUrl` | Avatar service URL |
| `Services__Voice__BaseUrl` | Voice service URL |
| `JwtConfiguration__PrivateKeyPath` | RSA private key for JWT signing |
| `Stripe__SecretKey` | Stripe payment API key |
| `Encryption__Key` | AES-256 encryption key |
| `CORS__AllowedOrigins` | Comma-separated allowed origins |

## Deployment Checklist

- [ ] All health checks green: `curl /health`
- [ ] Database migrations applied
- [ ] Environment variables set for target environment
- [ ] JWT keys generated and mounted
- [ ] Stripe webhook configured
- [ ] CORS origins updated for production domain
- [ ] Grafana dashboards loading data
- [ ] Alert rules active in Prometheus
- [ ] SSL/TLS certificates valid
