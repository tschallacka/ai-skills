#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <session-id>" >&2
    exit 64
fi

THREAD_ID="$1"
if [[ ! "$THREAD_ID" =~ ^[A-Za-z0-9_-]+$ ]]; then
    printf 'thread_id=%s\n' "$THREAD_ID"
    printf 'usage_records=0\n'
    printf 'total_usage_tokens=0\n'
    printf 'telemetry_status=unavailable:invalid thread id\n'
    exit 0
fi
if [ -n "${CODEX_HOME:-}" ]; then
    CODEX_HOME_DIR="$CODEX_HOME"
elif [ -n "${HOME:-}" ]; then
    CODEX_HOME_DIR="$HOME/.codex"
else
    printf 'thread_id=%s\n' "$THREAD_ID"
    printf 'usage_records=unavailable\n'
    printf 'total_usage_tokens=unavailable\n'
    printf 'telemetry_status=unavailable:CODEX_HOME and HOME are unset\n'
    exit 0
fi

printf 'thread_id=%s\n' "$THREAD_ID"

if [ ! -d "$CODEX_HOME_DIR" ]; then
    printf 'usage_records=unavailable\n'
    printf 'total_usage_tokens=unavailable\n'
    printf 'telemetry_status=unavailable:missing Codex home %s\n' "$CODEX_HOME_DIR"
    exit 0
fi

mapfile -t DB_CANDIDATES < <(
    {
        if [ -n "${CODEX_TELEMETRY_DB:-}" ]; then
            printf '%s\n' "$CODEX_TELEMETRY_DB"
        fi
        find "$CODEX_HOME_DIR" -maxdepth 2 -type f -name '*.sqlite' -print
    } | awk 'NF && !seen[$0]++' | sort
)

if command -v python3 >/dev/null 2>&1; then
if python3 - "$THREAD_ID" "${DB_CANDIDATES[@]}" <<'PY'
import re
import sqlite3
import sys

thread_id = sys.argv[1]
databases = sys.argv[2:]
log_result = None


def emit(database, records, total, source):
    print(f"telemetry_db={database}")
    print(f"usage_records={records}")
    print(f"total_usage_tokens={total}")
    print(f"telemetry_source={source}")
    print("telemetry_status=available")


for database in databases:
    try:
        connection = sqlite3.connect(f"file:{database}?mode=ro", uri=True)
    except sqlite3.Error:
        continue

    try:
        tables = {
            row[0]
            for row in connection.execute(
                "select name from sqlite_master where type='table'"
            )
        }

        # Newer Codex state stores keep the final cumulative total here.
        if "threads" in tables:
            row = connection.execute(
                "select tokens_used from threads where id = ?",
                (thread_id,),
            ).fetchone()
            if row is not None and row[0] is not None:
                emit(database, 1, int(row[0]), "threads.tokens_used")
                sys.exit(0)

        # Older/log-based stores keep cumulative snapshots in log bodies.
        if "logs" in tables:
            columns = {
                row[1]
                for row in connection.execute("pragma table_info(logs)")
            }
            if "thread_id" not in columns or "feedback_log_body" not in columns:
                continue

            values = []
            for (body,) in connection.execute(
                "select feedback_log_body from logs where thread_id = ? order by id",
                (thread_id,),
            ):
                if not body:
                    continue
                for first, second in re.findall(
                    r'"total_usage_tokens"\s*:\s*([0-9]+)|total_usage_tokens=([0-9]+)',
                    body,
                ):
                    values.extend(int(value) for value in (first, second) if value)

            if values and log_result is None:
                log_result = (
                    database,
                    len(values),
                    max(values),
                    "logs.feedback_log_body",
                )
    except sqlite3.Error:
        # Codex may keep an auxiliary database locked or unavailable while
        # another candidate remains readable.
        continue
    finally:
        connection.close()

if log_result is not None:
    emit(*log_result)
    sys.exit(0)

sys.exit(1)
PY
then
    exit 0
fi
fi

if command -v sqlite3 >/dev/null 2>&1; then
    SQLITE_LOG_DB=""
    SQLITE_LOG_RECORDS=0
    SQLITE_LOG_TOTAL=""
    for database in "${DB_CANDIDATES[@]}"; do
        THREAD_TOKENS="$(sqlite3 -batch -noheader "$database" \
            "pragma query_only=on; select tokens_used from threads where id = '$THREAD_ID' limit 1;" \
            2>/dev/null || true)"
        if [[ "$THREAD_TOKENS" =~ ^[0-9]+$ ]]; then
            printf 'telemetry_db=%s\n' "$database"
            printf 'usage_records=1\n'
            printf 'total_usage_tokens=%s\n' "$THREAD_TOKENS"
            printf 'telemetry_source=threads.tokens_used\n'
            printf 'telemetry_status=available\n'
            exit 0
        fi

        LOG_BODIES="$(sqlite3 -batch -noheader "$database" \
            "pragma query_only=on; select feedback_log_body from logs where thread_id = '$THREAD_ID' order by id;" \
            2>/dev/null || true)"
        LOG_VALUES="$(printf '%s\n' "$LOG_BODIES" |
            grep -oE '"total_usage_tokens"[[:space:]]*:[[:space:]]*[0-9]+|total_usage_tokens=[0-9]+' |
            sed -E 's/.*[:=][[:space:]]*//' || true)"
        if [ -n "$LOG_VALUES" ] && [ -z "$SQLITE_LOG_DB" ]; then
            SQLITE_LOG_DB="$database"
            SQLITE_LOG_RECORDS="$(printf '%s\n' "$LOG_VALUES" | awk 'NF { count++ } END { print count + 0 }')"
            SQLITE_LOG_TOTAL="$(printf '%s\n' "$LOG_VALUES" | sort -n | tail -1)"
        fi
    done

    if [ -n "$SQLITE_LOG_DB" ]; then
        printf 'telemetry_db=%s\n' "$SQLITE_LOG_DB"
        printf 'usage_records=%s\n' "$SQLITE_LOG_RECORDS"
        printf 'total_usage_tokens=%s\n' "$SQLITE_LOG_TOTAL"
        printf 'telemetry_source=logs.feedback_log_body\n'
        printf 'telemetry_status=available\n'
        exit 0
    fi
fi

ROLLOUT_FILE="$(find "$CODEX_HOME_DIR/sessions" -type f -name "*-${THREAD_ID}.jsonl" -print -quit 2>/dev/null || true)"
if [ -n "$ROLLOUT_FILE" ]; then
    ROLLOUT_RECORDS="$(grep -c '"type":"event_msg".*"type":"token_count"' "$ROLLOUT_FILE" 2>/dev/null || true)"
    ROLLOUT_TOTAL="$(sed -nE 's/.*"total_token_usage":\{[^}]*"total_tokens":([0-9]+).*/\1/p' "$ROLLOUT_FILE" | tail -1)"
    if [ "${ROLLOUT_RECORDS:-0}" -gt 0 ] && [ -n "$ROLLOUT_TOTAL" ]; then
        printf 'telemetry_db=unavailable\n'
        printf 'usage_records=%s\n' "$ROLLOUT_RECORDS"
        printf 'total_usage_tokens=%s\n' "$ROLLOUT_TOTAL"
        printf 'telemetry_source=rollout-jsonl:%s\n' "$ROLLOUT_FILE"
        printf 'telemetry_status=available\n'
        exit 0
    fi
fi

printf 'telemetry_db=unavailable\n'
printf 'usage_records=0\n'
printf 'total_usage_tokens=0\n'
printf 'telemetry_status=unavailable:no matching telemetry record found\n'
