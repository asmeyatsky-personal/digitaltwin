# Cloud Run Deployment

Per ADR-0002, services deploy to Cloud Run with Secret Manager + Workload Identity.

## Secrets provisioned per service

| Secret name                   | Contents                                |
|-------------------------------|-----------------------------------------|
| `identity-database-url`       | Postgres connection string (PGPASSWORD-free; use IAM auth) |
| `identity-jwt-private-key`    | RS256 private key (PEM)                 |
| `identity-jwt-public-key`     | RS256 public key (PEM)                  |

## One-time project bootstrap

```sh
gcloud iam service-accounts create identity-service-sa \
    --description="Runs the identity Cloud Run service"

gcloud secrets add-iam-policy-binding identity-database-url \
    --member="serviceAccount:identity-service-sa@${PROJECT_ID}.iam.gserviceaccount.com" \
    --role="roles/secretmanager.secretAccessor"

# Repeat for each secret.
```

## Build + deploy

```sh
REGION=us-central1
PROJECT_ID=digitaltwin-prod
COMMIT_SHA=$(git rev-parse --short HEAD)

gcloud builds submit backend \
    --config=deploy/cloud-run/cloudbuild.yaml \
    --substitutions=_COMMIT_SHA=${COMMIT_SHA},_REGION=${REGION}

sed -e "s|PROJECT_ID|${PROJECT_ID}|g" \
    -e "s|REGION|${REGION}|g" \
    -e "s|COMMIT_SHA|${COMMIT_SHA}|g" \
    deploy/cloud-run/identity-service.yaml \
    | gcloud run services replace - --region=${REGION}
```

## Retiring the Kubernetes manifests

The `k8s/` directory is retained only as reference during migration. Each
service that ships to Cloud Run deletes its corresponding K8s deployment in
the same PR.
