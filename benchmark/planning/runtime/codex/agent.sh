#!/usr/bin/env bash
# Codex benchmark driver. Implements the reserved runtime contract using the
# exact pre-refactor Codex behavior (argv, session-id, SQLite telemetry).

set -euo pipefail

AGENT_NAME="codex"
agent_bin="codex"
agent_model_env="CODEX_MODEL"
agent_default_model="gpt-5.5"

_agent_codex_argv() {
    # shared caps/workspace open flags for codex exec
    AGENT_ARGV=(
        "$1" -a never exec --model "$(agent_resolve_model)" --json
        -C "$2"
        --skip-git-repo-check
        --sandbox workspace-write
        --add-dir "$3"
        --add-dir "$2"
        "$(cat "$4")"
    )
}

agent_argv_worker() {
    # $1 = workspace (BENCH_ROOT), $2 = capsule, $3 = prompt
    _agent_codex_argv "${agent_bin:-codex}" "$1" "$2" "$3"
}

agent_argv_reviewer() {
    # $1 = workspace, $2 = capsule, $3 = prompt, $4 (optional) = binary override
    local binary="${4:-${agent_bin:-codex}}"
    _agent_codex_argv "$binary" "$1" "$2" "$3"
}

agent_argv_analyzer() {
    # $1 = workspace, $2 = capsule, $3 = prompt
    _agent_codex_argv "${agent_bin:-codex}" "$1" "$2" "$3"
}

agent_session_id() {
    # $1 = worker output jsonl; port of session-id-from-jsonl.sh.
    [ "$#" -eq 1 ] || return 64
    sed -nE \
        -e 's/.*"type"[[:space:]]*:[[:space:]]*"thread.started".*"thread_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
        -e 's/.*"thread_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
        -e 's/.*"threadId"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
        -e 's/.*"conversation_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
        -e 's/.*"session_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
        "$1" | head -1
}

agent_telemetry() {
    # $1 = session id. Port of telemetry.sh SQLite parser; honest unavailable
    # fallback, never fabricates token numbers.
    local thread_id="$1"
    local cod_home_dir cod_home="${CODEX_HOME:-}"

    if [ -n "$cod_home" ]; then
        cod_home_dir="$cod_home"
    elif [ -n "${HOME:-}" ]; then
        cod_home_dir="$HOME/.codex"
    else
        printf 'thread_id=%s\n' "$thread_id"
        printf 'usage_records=unavailable\n'
        printf 'total_usage_tokens=unavailable\n'
        printf 'telemetry_status=unavailable:CODEX_HOME and HOME are unset\n'
        return 0
    fi

    printf 'thread_id=%s\n' "$thread_id"

    if [ ! -d "$cod_home_dir" ]; then
        printf 'usage_records=unavailable\n'
        printf 'total_usage_tokens=unavailable\n'
        printf 'telemetry_status=unavailable:missing Codex home %s\n' "$cod_home_dir"
        return 0
    fi

    local -a db_candidates=()
    mapfile -t db_candidates < <(
        {
            if [ -n "${CODEX_TELEMETRY_DB:-}" ]; then
                printf '%s\n' "$CODEX_TELEMETRY_DB"
            fi
            find "$cod_home_dir" -maxdepth 2 -type f -name '*.sqlite' -print
        } | awk 'NF && !seen[$0]++' | sort
    )

    if command -v python3 >/dev/null 2>&1; then
    if python3 - "$thread_id" "${db_candidates[@]}" <<'PY'
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

        if "threads" in tables:
            row = connection.execute(
                "select tokens_used from threads where id = ?",
                (thread_id,),
            ).fetchone()
            if row is not None and row[0] is not None:
                emit(database, 1, int(row[0]), "threads.tokens_used")
                sys.exit(0)

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
        continue
    finally:
        connection.close()

if log_result is not None:
    emit(*log_result)
    sys.exit(0)

sys.exit(1)
PY
    then
        return 0
    fi
    fi

    if command -v sqlite3 >/dev/null 2>&1; then
        local sqlite_log_db="" sqlite_log_records=0 sqlite_log_total="" database thread_tokens log_bodies log_values
        for database in "${db_candidates[@]}"; do
            thread_tokens="$(sqlite3 -batch -noheader "$database" \
                "pragma query_only=on; select tokens_used from threads where id = '$thread_id' limit 1;" \
                2>/dev/null || true)"
            if [[ "$thread_tokens" =~ ^[0-9]+$ ]]; then
                printf 'telemetry_db=%s\n' "$database"
                printf 'usage_records=1\n'
                printf 'total_usage_tokens=%s\n' "$thread_tokens"
                printf 'telemetry_source=threads.tokens_used\n'
                printf 'telemetry_status=available\n'
                return 0
            fi

            log_bodies="$(sqlite3 -batch -noheader "$database" \
                "pragma query_only=on; select feedback_log_body from logs where thread_id = '$thread_id' order by id;" \
                2>/dev/null || true)"
            log_values="$(printf '%s\n' "$log_bodies" |
                grep -oE '"total_usage_tokens"[[:space:]]*:[[:space:]]*[0-9]+|total_usage_tokens=[0-9]+' |
                sed -E 's/.*[:=][[:space:]]*//' || true)"
            if [ -n "$log_values" ] && [ -z "$sqlite_log_db" ]; then
                sqlite_log_db="$database"
                sqlite_log_records="$(printf '%s\n' "$log_values" | awk 'NF { count++ } END { print count + 0 }')"
                sqlite_log_total="$(printf '%s\n' "$log_values" | sort -n | tail -1)"
            fi
        done

        if [ -n "$sqlite_log_db" ]; then
            printf 'telemetry_db=%s\n' "$sqlite_log_db"
            printf 'usage_records=%s\n' "$sqlite_log_records"
            printf 'total_usage_tokens=%s\n' "$sqlite_log_total"
            printf 'telemetry_source=logs.feedback_log_body\n'
            printf 'telemetry_status=available\n'
            return 0
        fi
    fi

    local rollout_file rollout_records rollout_total
    rollout_file="$(find "$cod_home_dir/sessions" -type f -name "*-${thread_id}.jsonl" -print -quit 2>/dev/null || true)"
    if [ -n "$rollout_file" ]; then
        rollout_records="$(grep -c '"type":"event_msg".*"type":"token_count"' "$rollout_file" 2>/dev/null || true)"
        rollout_total="$(sed -nE 's/.*"total_token_usage":\{[^}]*"total_tokens":([0-9]+).*/\1/p' "$rollout_file" | tail -1)"
        if [ "${rollout_records:-0}" -gt 0 ] && [ -n "$rollout_total" ]; then
            printf 'telemetry_db=unavailable\n'
            printf 'usage_records=%s\n' "$rollout_records"
            printf 'total_usage_tokens=%s\n' "$rollout_total"
            printf 'telemetry_source=rollout-jsonl:%s\n' "$rollout_file"
            printf 'telemetry_status=available\n'
            return 0
        fi
    fi

    printf 'telemetry_db=unavailable\n'
    printf 'usage_records=0\n'
    printf 'total_usage_tokens=0\n'
    printf 'telemetry_status=unavailable:no matching telemetry record found\n'
}