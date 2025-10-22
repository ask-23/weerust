#!/usr/bin/env bash
set -euo pipefail

# Usage: scripts/deploy-cloudrun.sh <gcp-project> <region> <image>
PROJECT=${1:?project id}
REGION=${2:?region}
IMAGE=${3:?image reference}
SERVICE=${SERVICE_NAME:-weewx-rs}

gcloud config set project "$PROJECT"
gcloud run deploy "$SERVICE" \
  --image "$IMAGE" \
  --region "$REGION" \
  --platform managed \
  --allow-unauthenticated \
  --port 8080 \
  --concurrency 40 \
  --cpu 1 \
  --memory 256Mi \
  --min-instances 0 \
  --max-instances 10 \
  --timeout 10s \
  --ingress all

echo "Deployed. Visit the Cloud Run console to get the URL."

