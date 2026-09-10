#!/usr/bin/env bash
# MODE: DEV
# test-agent-identity-plugin.sh — agent-identity-plugin's two hooks, driven
# directly with synthetic PreToolUse/SubagentStart payloads (the shape Claude
# Code itself is measured to send, per src/agent-session-key/HARNESS-IDENTITY.md).
#
# This proves the hook scripts' own logic: what a plugin.json/hooks.json
# registration triggers Claude Code to invoke. It cannot prove Claude Code
# itself calls them with these exact payloads in a live session — that half
# is the measurement already on file, not something a test script can drive.
#
# Usage: test-agent-identity-plugin.sh
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$root/planning/tests/lib-test.sh"
t_begin

export LC_ALL=C
work="$(mktemp -d "${TMPDIR:-/tmp}/agent-identity-plugin.XXXXXX")"
trap 'rm -rf "$work"' EXIT

RJQ="$root/target/release/rjq"
if ! command -v cargo >/dev/null 2>&1; then
    prebuilt_rjq="$(ls "$root"/planning/bin/*/rjq 2>/dev/null | head -1 || true)"
    if [ -n "$prebuilt_rjq" ]; then
        RJQ="$prebuilt_rjq"
    else
        t_skip 'test-agent-identity-plugin: no cargo and no prebuilt rjq - hook assertions did not run'
    fi
else
    ( cd "$root/src/rjq" && cargo build --release >/dev/null 2>&1 ) || t_fail "cargo build rjq failed"
fi

plugin="$root/agent-identity-plugin"
export PATH="$(dirname "$RJQ"):$PATH"
export AI_SKILLS_AGENT_IDENTITY_DIR="$work/register"

run_subagent_start() { # <payload json>
    printf '%s' "$1" | "$BASH" "$plugin/hooks/subagent-start.sh"
}

run_pre_tool_use() { # <payload json>
    printf '%s' "$1" | "$BASH" "$plugin/hooks/pre-tool-use.sh"
}

# ── SubagentStart: injects the id and type, names them by name ─────────────
out="$(run_subagent_start '{"agent_id":"a4631ac048e789cbb","agent_type":"code-researcher"}')"
context="$(printf '%s' "$out" | "$RJQ" -r '.hookSpecificOutput.additionalContext')"
case "$context" in
    *a4631ac048e789cbb*) ;;
    *) t_fail "SubagentStart did not name the agent id in its context: $context" ;;
esac
case "$context" in
    *code-researcher*) ;;
    *) t_fail "SubagentStart did not name the agent type in its context: $context" ;;
esac
hook_event="$(printf '%s' "$out" | "$RJQ" -r '.hookSpecificOutput.hookEventName')"
t_assert_eq 'SubagentStart names its own event' "$hook_event" 'SubagentStart'

# ── SubagentStart: a null agent_id (the main agent) reads as "main" ────────
out="$(run_subagent_start '{"agent_id":null,"agent_type":null}')"
context="$(printf '%s' "$out" | "$RJQ" -r '.hookSpecificOutput.additionalContext')"
case "$context" in
    *'agent main'*) ;;
    *) t_fail "a null agent_id was not read as the main agent: $context" ;;
esac

# ── PreToolUse: writes one register line, keyed by session id ──────────────
rm -rf "$AI_SKILLS_AGENT_IDENTITY_DIR"
run_pre_tool_use '{"session_id":"sess-1","tool_use_id":"toolu_abc","agent_id":"worker-1","agent_type":"code-researcher"}' >/dev/null
register="$AI_SKILLS_AGENT_IDENTITY_DIR/sess-1.jsonl"
[ -f "$register" ] || t_fail "PreToolUse did not create the register file $register"
line="$(cat "$register" 2>/dev/null || true)"
t_assert_eq 'the register line names the tool_use_id' \
    "$(printf '%s' "$line" | "$RJQ" -r .tool_use_id)" 'toolu_abc'
t_assert_eq 'the register line names the agent_id' \
    "$(printf '%s' "$line" | "$RJQ" -r .agent_id)" 'worker-1'

# ── PreToolUse: a second call appends, never rewrites ───────────────────────
run_pre_tool_use '{"session_id":"sess-1","tool_use_id":"toolu_def","agent_id":"worker-2","agent_type":"code-researcher"}' >/dev/null
line_count="$(wc -l < "$register" | tr -d ' ')"
t_assert_eq 'a second call appends a second line, not a rewrite' "$line_count" '2'

# ── PreToolUse: a null agent_id (the main agent) is recorded as "main" ─────
run_pre_tool_use '{"session_id":"sess-1","tool_use_id":"toolu_main","agent_id":null,"agent_type":null}' >/dev/null
main_agent="$(grep -F 'toolu_main' "$register" | "$RJQ" -r .agent_id)"
t_assert_eq 'a null agent_id is recorded as the literal string main' "$main_agent" 'main'

# ── PreToolUse: missing session_id or tool_use_id writes nothing ───────────
before="$(wc -l < "$register" | tr -d ' ')"
run_pre_tool_use '{"agent_id":"x"}' >/dev/null
after="$(wc -l < "$register" | tr -d ' ')"
t_assert_eq 'a payload with neither key writes no register line' "$before" "$after"

# ── The two sides agree on the register path (the one thing that must never
#    drift silently — see agent-identity-plugin/README.md) ─────────────────
sessions_dir="$AI_SKILLS_AGENT_IDENTITY_DIR"
run_pre_tool_use '{"session_id":"sess-2","tool_use_id":"toolu_join","agent_id":"joiner","agent_type":"worker"}' >/dev/null
[ -f "$sessions_dir/sess-2.jsonl" ] || t_fail "the register path the hook wrote to did not match what the plugin documents computing"

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-agent-identity-plugin: PASS'
