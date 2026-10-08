#!/usr/bin/env bash
# MODE: DEV
# test-editor-gate-context.sh -- editor_gate_emit_context, the shared
# safe-JSON builder behind the Edit/Write and Read reminders. Exercises the
# exact failure class a hand-rolled printf string has: a note containing a
# quote or an apostrophe.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
plugin_dir="$(cd "$tests_dir/.." && pwd)"
repo_root="$(cd "$plugin_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

# shellcheck source=editor-gate-plugin/hooks/lib.sh
source "$plugin_dir/hooks/lib.sh"

# jq is this test's stand-in for rjq (dev tooling, per AGENTS.md): both speak
# the same `-n -c --arg name value <filter>` contract, so exercising
# editor_gate_emit_context against jq proves the same escaping behavior the
# shipped rjq provides, without requiring rjq to be installed just to run
# this suite.
jq_bin="$(command -v jq || true)"
if [ -z "$jq_bin" ]; then
    t_skip 'jq not found on PATH -- nothing to exercise editor_gate_emit_context against'
fi

note_with_apostrophe="Edit's own match check only confirms the string is unique."
output="$(editor_gate_emit_context "$jq_bin" "$note_with_apostrophe")"

t_assert_eq 'emits valid JSON even with an apostrophe in the note' \
    "$(printf '%s' "$output" | "$jq_bin" -e . >/dev/null 2>&1 && echo valid || echo invalid)" \
    'valid'

t_assert_eq 'additionalContext round-trips the apostrophe-bearing note exactly' \
    "$(printf '%s' "$output" | "$jq_bin" -r '.hookSpecificOutput.additionalContext')" \
    "$note_with_apostrophe"

t_assert_eq 'permissionDecision is always allow' \
    "$(printf '%s' "$output" | "$jq_bin" -r '.hookSpecificOutput.permissionDecision')" \
    'allow'

t_assert_eq 'hookEventName is PreToolUse' \
    "$(printf '%s' "$output" | "$jq_bin" -r '.hookSpecificOutput.hookEventName')" \
    'PreToolUse'

note_with_quote='A "quoted" word and a backslash \ in one note.'
output_quoted="$(editor_gate_emit_context "$jq_bin" "$note_with_quote")"

t_assert_eq 'emits valid JSON with an embedded double quote and backslash' \
    "$(printf '%s' "$output_quoted" | "$jq_bin" -e . >/dev/null 2>&1 && echo valid || echo invalid)" \
    'valid'

t_assert_eq 'additionalContext round-trips embedded quote and backslash exactly' \
    "$(printf '%s' "$output_quoted" | "$jq_bin" -r '.hookSpecificOutput.additionalContext')" \
    "$note_with_quote"

t_end
