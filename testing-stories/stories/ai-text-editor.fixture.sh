#!/usr/bin/env bash
# Seeds a small, realistic config file with a deliberate bug for the
# ai-text-editor testing story to find and fix. Sourced with /workspace as
# cwd, before the agent starts. Idempotent, no network.
set -euo pipefail

cat > retry-policy.conf <<'EOF'
# Service retry policy — one section per client.
# Format: max_retries, backoff_ms, timeout_ms per client block.

[client.billing]
max_retries = 3
backoff_ms = 200
timeout_ms = 5000

[client.inventory]
max_retries = 3
backoff_ms = 200
timeout_ms = 5000

[client.notifications]
max_retries = 3
backoff_ms = 200
timeout_ms = 1500

[client.search]
max_retries = 3
backoff_ms = 200
timeout_ms = 5000

[client.audit]
max_retries = 3
backoff_ms = 200
timeout_ms = 5000
EOF
