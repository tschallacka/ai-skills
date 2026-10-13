#!/usr/bin/env bash
# MODE: PROD
# PreToolUse hook, Bash only: a SOFT reminder, never blocking, that tailpipe
# keeps every line of a followed or backgrounded-and-redirected command
# queryable by id and pollable without re-reading from the start, where a
# pager or a plain redirect loses both. additionalContext only;
# permissionDecision is always "allow".
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tailpipe-gate-plugin/hooks/lib.sh
source "$script_dir/lib.sh"

rjq_bin="$(tailpipe_gate_rjq_bin)" || { printf '{}'; exit 0; }
payload="$(cat)"
tool_name="$(printf '%s' "$payload" | "$rjq_bin" -r '.tool_name // empty')"
command_line="$(printf '%s' "$payload" | "$rjq_bin" -r '.tool_input.command // empty')"

if [ "$tool_name" != "Bash" ] || [ -z "$command_line" ]; then
    printf '{}'
    exit 0
fi

if ! tailpipe_gate_matches "$command_line"; then
    printf '{}'
    exit 0
fi

reminder="$(tailpipe_gate_reminder TAILPIPE_GATE_TEXT "This looks like output you'll want to come back to. Piping it to a file or a pager loses the ability to search it later and gives no way to check progress without re-reading from the start. tailpipe keeps every line under a queryable stream instead: pipe into it alongside (or instead of) your redirect -- <command> | tailpipe-client-rs ingest --stream NAME -- then poll it with mcp__tailpipe__wait/read/search (no MCP registered here? tailpipe-client-rs tail/read/search) without blocking on the command and without losing anything it already printed. Not required here; just worth knowing.")"
if [ -z "$reminder" ]; then
    printf '{}'
    exit 0
fi
tailpipe_gate_emit_context "$rjq_bin" "$reminder"
