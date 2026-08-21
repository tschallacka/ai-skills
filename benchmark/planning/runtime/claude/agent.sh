#!/usr/bin/env bash
# Claude Code benchmark driver. Implements the reserved runtime contract for
# the `claude` CLI in non-interactive print mode.
#
# Capsule-read mechanism: `claude -p` with `--add-dir` for the isolated
# workspace and capsule, matching claude's native allow-list tool-access model.
# `--permission-mode acceptEdits` auto-accepts file reads/writes inside the
# allowed dirs so a non-interactive worker can produce plan artifacts without
# prompting (claude cannot prompt in `-p` mode and otherwise denies writes).
# `--allowedTools "Bash(...validate-plan.sh)"` is the *worker's* sole legitimate
# shell command (running the tagged validator); everything else stays denied.
# The allowlist is role-scoped: only the worker capsule contains
# planning/scripts/, so the reviewer and analyzer roles pass no --allowedTools
# at all (allowlisting a path absent from their capsule advertised a capability
# they cannot have and denied nothing extra).
#
# cwd: claude has no cwd flag of its own (`--add-dir` grants access, it does not
# chdir), so the driver reports the workspace through AGENT_CWD and
# launch_agent runs the agent there — the same cwd codex gets from `-C` and
# opencode from `--dir`.
#
# Model env decision (recorded W14): the harness override env is CLAUDE_MODEL
# (consistent with CODEX_MODEL / OPENCODE_MODEL). claude itself also honours
# ANTHROPIC_MODEL as a native default, but the harness always passes
# `--model "$(agent_resolve_model)"` explicitly, so CLAUDE_MODEL is the single
# authoritative override and defaults to the `opus` alias.
#
# Session id comes from claude's single JSON result (`session_id`). Telemetry
# aggregates token usage from the on-disk session transcript under
# ~/.claude/projects/<escaped-cwd>/<session-id>.jsonl (assistant.message.usage);
# when no transcript is found it degrades honestly to unavailable.

set -euo pipefail

AGENT_NAME="claude"
agent_bin="claude"
agent_model_env="CLAUDE_MODEL"
agent_default_model="opus"

_agent_claude_argv() {
    # $1 = binary, $2 = workspace, $3 = capsule, $4 = prompt, $5 = role
    [ "$#" -eq 5 ] || return 64
    local -a allowed=()
    if [ "$5" = worker ]; then
        allowed=(--allowedTools "Bash($3/planning/scripts/validate-plan.sh)")
    fi
    AGENT_CWD="$2"
    AGENT_ARGV=(
        "$1" -p
        --output-format=json
        --add-dir "$2"
        --add-dir "$3"
        --permission-mode acceptEdits
        ${allowed[@]+"${allowed[@]}"}
        --model "$(agent_resolve_model)"
        "$(cat "$4")"
    )
}

agent_argv_worker() {
    # $1 = workspace, $2 = capsule, $3 = prompt
    _agent_claude_argv "${agent_bin:-claude}" "$1" "$2" "$3" worker
}

agent_argv_reviewer() {
    # $1 = workspace, $2 = capsule, $3 = prompt, $4 (optional) = binary override
    local binary="${4:-${agent_bin:-claude}}"
    _agent_claude_argv "$binary" "$1" "$2" "$3" reviewer
}

agent_argv_analyzer() {
    # $1 = workspace, $2 = capsule, $3 = prompt
    _agent_claude_argv "${agent_bin:-claude}" "$1" "$2" "$3" analyzer
}

agent_session_id() {
    # $1 = claude output stream; prints the first session_id it carries.
    # Line-oriented: the agent's stderr is merged into this JSONL, so a
    # whole-file parse breaks on one warning. A missing file degrades to empty.
    [ "$#" -eq 1 ] || return 64
    [ -f "$1" ] || return 0
    # `sed -n '1p'`, not `head -1`: head closes the pipe early, which is
    # SIGPIPE for sed and a failure under pipefail.
    sed -nE 's/.*"session_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$1" 2>/dev/null |
        sed -n '1p'
}

agent_telemetry() {
    # $1 = session id; honest unavailable when no claude transcript matches.
    local session_id="$1" transcript=""
    transcript="$(find "${CLAUDE_PROJECTS_DIR:-$HOME/.claude/projects}" -type f -name "${session_id}.jsonl" -print -quit 2>/dev/null || true)"
    printf 'thread_id=%s\n' "$session_id"
    if [ -z "$transcript" ]; then
        printf 'usage_records=0\n'
        printf 'total_usage_tokens=0\n'
        printf 'telemetry_status=unavailable:no claude session transcript found\n'
        return 0
    fi
    if ! command -v python3 >/dev/null 2>&1; then
        printf 'usage_records=unavailable\n'
        printf 'total_usage_tokens=unavailable\n'
        printf 'telemetry_status=unavailable:python3 required to read claude transcript\n'
        return 0
    fi
    if python3 - "$transcript" <<'PY'
import json
import sys

transcript = sys.argv[1]
records = 0
total = 0
try:
    with open(transcript, encoding="utf-8") as handle:
        for line in handle:
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("type") != "assistant":
                continue
            usage = (event.get("message") or {}).get("usage")
            if not isinstance(usage, dict):
                continue
            total += sum(
                usage.get(key) or 0
                for key in (
                    "input_tokens",
                    "cache_creation_input_tokens",
                    "cache_read_input_tokens",
                    "output_tokens",
                )
            )
            records += 1
except OSError:
    sys.exit(2)

if records == 0:
    sys.exit(1)

print(f"telemetry_db={transcript}")
print(f"usage_records={records}")
print(f"total_usage_tokens={total}")
print("telemetry_source=claude-transcript-assistant-usage")
print("telemetry_status=available")
sys.exit(0)
PY
    then
        return 0
    fi
    printf 'usage_records=0\n'
    printf 'total_usage_tokens=0\n'
    printf 'telemetry_status=unavailable:no usable claude usage in session transcript\n'
}