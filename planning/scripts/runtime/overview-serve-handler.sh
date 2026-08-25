#!/usr/bin/env bash
# MODE: PROD
# overview-serve-handler.sh — HTTP responses for the served overview.
# Called by the runtime-specific server with:
#   $1 = plan directory
# Outputs the response body to stdout based on the ROUTE environment variable.
set -euo pipefail
export LC_ALL=C

plan_dir="${PLAN_DIR:?}"
route="${ROUTE:-/}"

case "$route" in
    /state.json|/state)
        bash "$(dirname "${BASH_SOURCE[0]}")/../overview-state.sh" "$plan_dir"
        ;;
    /|/index.html)
        OVERVIEW_NOW="$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
            bash "$(dirname "${BASH_SOURCE[0]}")/../render-plan-overview.sh" "$plan_dir" --serve
        ;;
    *)
        printf 'Not found: %s\n' "$route" >&2
        exit 1
        ;;
esac
