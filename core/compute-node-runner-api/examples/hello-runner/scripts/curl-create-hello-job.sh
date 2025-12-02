#!/usr/bin/env bash
set -euo pipefail

DMS_BASE_URL=${DMS_BASE_URL:-https://dms.dev.aukiverse.com/v1}
APP_JWT=${APP_JWT:-}
DOMAIN_ID=${DOMAIN_ID:-}
OUTPUTS_PREFIX=${OUTPUTS_PREFIX:-hello-demo/}
MESSAGE=${MESSAGE:-Hello from curl-create-hello-job.sh}
TASK_LABEL=${TASK_LABEL:-hello}
STAGE=${STAGE:-hello}
BILLING_UNITS=${BILLING_UNITS:-1}
INPUT_CID=${INPUT_CID:-} # https://domain.dev.aukiverse.com/api/v1/domains/<DOMAIN_ID>/data/<DATA_ID
CAPABILITY="/examples/hello/v1"

if [[ -z "$APP_JWT" ]]; then
  echo "error: set APP_JWT to a DDS-signed app token with domain:rw scope" >&2
  exit 1
fi

if [[ -z "$DOMAIN_ID" ]]; then
  echo "error: set DOMAIN_ID to the target domain UUID" >&2
  exit 1
fi

PAYLOAD=$(cat <<EOF
{
  "label": "hello-runner-demo",
  "domain_id": "$DOMAIN_ID",
  "priority": 0,
  "meta": {},
  "tasks": [
    {
      "label": "$TASK_LABEL",
      "stage": "$STAGE",
      "capability": "$CAPABILITY",
      "capability_filters": {},
      "priority": 0,
      "inputs_cids": ["$INPUT_CID"],
      "outputs_prefix": "$OUTPUTS_PREFIX",
      "mode": "public",
      "meta": { "message": "$MESSAGE" },
      "max_attempts": 1
    }
  ],
  "edges": []
}
EOF
)

URL="${DMS_BASE_URL%/}/jobs"

response=$(curl -sS -w "\n%{http_code}" -X POST "$URL" \
  -H "Authorization: Bearer ${APP_JWT}" \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD")

body=${response%$'\n'*}
status=${response##*$'\n'}

echo "HTTP $status"
echo "$body" | (command -v jq >/dev/null 2>&1 && jq . || cat)
