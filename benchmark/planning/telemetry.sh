#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <session-id>" >&2
    exit 64
fi

THREAD_ID="$1"
DB="${CODEX_TELEMETRY_DB:-/home/tschallacka/.codex/logs_2.sqlite}"

printf 'thread_id=%s\n' "$THREAD_ID"

if [ ! -f "$DB" ]; then
    printf 'usage_records=unavailable\n'
    printf 'total_usage_tokens=unavailable\n'
    printf 'telemetry_status=unavailable:missing sqlite db %s\n' "$DB"
    exit 0
fi

if ! command -v sqlite3 >/dev/null 2>&1; then
    printf 'usage_records=unavailable\n'
    printf 'total_usage_tokens=unavailable\n'
    printf 'telemetry_status=unavailable:sqlite3 not found\n'
    exit 0
fi

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

if ! sqlite3 "$DB" \
    "select feedback_log_body from logs where thread_id = '$THREAD_ID' order by id;" > "$tmp" 2>/dev/null; then
    printf 'usage_records=unavailable\n'
    printf 'total_usage_tokens=unavailable\n'
    printf 'telemetry_status=unavailable:sqlite query failed\n'
    exit 0
fi

records="$(grep -oE '"total_usage_tokens"[[:space:]]*:[[:space:]]*[0-9]+' "$tmp" | wc -l | tr -d ' ')"
total="$(
    grep -oE '"total_usage_tokens"[[:space:]]*:[[:space:]]*[0-9]+' "$tmp" |
        sed -E 's/.*:[[:space:]]*//' |
        awk '{ sum += $1 } END { print sum + 0 }'
)"

printf 'usage_records=%s\n' "$records"
printf 'total_usage_tokens=%s\n' "$total"
if [ "$records" = 0 ]; then
    printf 'telemetry_status=unavailable:no matching usage records\n'
fi
