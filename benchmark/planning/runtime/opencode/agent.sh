#!/usr/bin/env bash
# opencode benchmark driver. Implements the reserved runtime contract for the
# opencode CLI (`opencode run`).
#
# Capsule-read mechanism: the worker/reviewer/analyzer run with `--dir` set to
# the isolated workspace and attach the key capsule files with `-f` so the
# prompt can reference them; `--auto` approves the explicit file reads inside
# the isolated roots. This is opencode's native mechanism (no codex-shaped
# sandbox flags are faked).
#
# Session id comes from opencode's JSON event stream (`sessionID`).
# Telemetry reads opencode's own SQLite session store (opencode.db) by session
# id; when no store/row is found it degrades honestly to unavailable.

set -euo pipefail

AGENT_NAME="opencode"
agent_bin="opencode"
agent_model_env="OPENCODE_MODEL"
agent_default_model="opencode/big-pickle"

_agent_opencode_attachments() {
    # $1 = capsule dir; prints -f flag pairs for the capsule reference files.
    local capsule="$1" file
    for file in task-spec.md planning/SKILL.md planning/REVIEWER.md basic-test-proof-plan.md; do
        if [ -f "$capsule/$file" ]; then
            printf -- '-f\n%s\n' "$capsule/$file"
        fi
    done
}

_agent_opencode_argv() {
    # $1 = binary, $2 = workspace, $3 = capsule, $4 = prompt
    local -a attachments=()
    local file
    while IFS= read -r line; do
        attachments+=("$line")
    done < <(_agent_opencode_attachments "$3")
    AGENT_ARGV=(
        "$1" run --format json
        --dir "$2"
        --model "$(agent_resolve_model)"
        "${attachments[@]}"
        --auto
        "$(cat "$4")"
    )
}

agent_argv_worker() {
    # $1 = workspace, $2 = capsule, $3 = prompt
    _agent_opencode_argv "${agent_bin:-opencode}" "$1" "$2" "$3"
}

agent_argv_reviewer() {
    # $1 = workspace, $2 = capsule, $3 = prompt, $4 (optional) = binary override
    local binary="${4:-${agent_bin:-opencode}}"
    _agent_opencode_argv "$binary" "$1" "$2" "$3"
}

agent_argv_analyzer() {
    # $1 = workspace, $2 = capsule, $3 = prompt
    _agent_opencode_argv "${agent_bin:-opencode}" "$1" "$2" "$3"
}

agent_session_id() {
    # $1 = opencode JSON event stream; first sessionID wins.
    [ "$#" -eq 1 ] || return 64
    sed -nE 's/.*"sessionID"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$1" | head -1
}

_agent_opencode_db() {
    local base
    for base in "${XDG_DATA_HOME:-$HOME/.local/share}/opencode" "$HOME/.local/share/opencode"; do
        if [ -f "$base/opencode.db" ]; then
            printf '%s/opencode.db' "$base"
            return 0
        fi
    done
    return 1
}

agent_telemetry() {
    # $1 = session id; honest unavailable when the opencode store has no row.
    local session_id="$1" db
    db="$(_agent_opencode_db)" || db=""
    printf 'thread_id=%s\n' "$session_id"
    if [ -z "$db" ]; then
        printf 'usage_records=unavailable\n'
        printf 'total_usage_tokens=unavailable\n'
        printf 'telemetry_status=unavailable:opencode.db not found\n'
        return 0
    fi
    if ! command -v python3 >/dev/null 2>&1; then
        printf 'usage_records=unavailable\n'
        printf 'total_usage_tokens=unavailable\n'
        printf 'telemetry_status=unavailable:python3 required to read opencode.db\n'
        return 0
    fi
    if python3 - "$db" "$session_id" <<'PY'
import sqlite3
import sys

db, session_id = sys.argv[1:3]
try:
    conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    row = conn.execute(
        "select tokens_input, tokens_output, tokens_reasoning,"
        " tokens_cache_read, tokens_cache_write from session where id = ?",
        (session_id,),
    ).fetchone()
    conn.close()
except sqlite3.Error:
    row = None
if row is None:
    sys.exit(1)
parts = [value for value in row if isinstance(value, int) and value >= 0]
total = sum(parts)
print(f"telemetry_db={db}")
print(f"usage_records=1")
print(f"total_usage_tokens={total}")
print("telemetry_source=session.tokens_*")
print("telemetry_status=available")
sys.exit(0)
PY
    then
        return 0
    fi
    printf 'usage_records=0\n'
    printf 'total_usage_tokens=0\n'
    printf 'telemetry_status=unavailable:no matching session record found in opencode.db\n'
}