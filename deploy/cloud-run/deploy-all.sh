#!/usr/bin/env bash
# Deploy every Rust service in backend/services/* to Cloud Run.
# Expects: PROJECT_ID, REGION, COMMIT_SHA.

set -euo pipefail

PROJECT_ID="${PROJECT_ID:?PROJECT_ID required}"
REGION="${REGION:-us-central1}"
COMMIT_SHA="${COMMIT_SHA:-$(git rev-parse --short HEAD)}"

CONTEXTS=(
    identity conversation emotion memory family achievement notification
    avatar voice community moderation therapy learning creative
)

# Each context has its own SECRET_PREFIX — uppercased context name with
# underscores. Override per context as needed.
for ctx in "${CONTEXTS[@]}"; do
    service="${ctx}-service"
    prefix=$(echo "$ctx" | tr '[:lower:]' '[:upper:]' | tr '-' '_')

    if [[ -f "deploy/cloud-run/${service}.yaml" ]]; then
        manifest="deploy/cloud-run/${service}.yaml"
    else
        manifest=$(mktemp)
        sed -e "s|SERVICE|${service}|g" \
            -e "s|SECRET_PREFIX|${prefix}|g" \
            -e "s|PROJECT_ID|${PROJECT_ID}|g" \
            -e "s|REGION|${REGION}|g" \
            -e "s|COMMIT_SHA|${COMMIT_SHA}|g" \
            deploy/cloud-run/_template.yaml > "$manifest"
    fi

    echo "==> deploying ${service}"
    gcloud run services replace "$manifest" --region="${REGION}" --project="${PROJECT_ID}"
done

echo "all services deployed"
