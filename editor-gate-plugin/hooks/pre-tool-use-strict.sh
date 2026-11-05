#!/usr/bin/env bash
# MODE: PROD
# PreToolUse hook for strict mode, which the installer turns on with a marker
# file: `ai-text-editor strict-gate` decides, this hook only hands it the call.
# Does nothing without the marker. With it, every failure to get an answer is a
# deny, because a hook that dies lets the tool call through.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=editor-gate-plugin/hooks/lib.sh
source "$script_dir/lib.sh"

deny_closed() { # <reason, free of quotes and backslashes>
    printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"%s"}}' "$1"
}

if ! editor_gate_is_strict; then
    cat >/dev/null
    printf '{}'
    exit 0
fi

if ! editor_bin="$(editor_gate_editor_bin)"; then
    cat >/dev/null
    deny_closed 'editor-gate strict mode: the ai-text-editor client is missing from its installed location; re-run the installer. Denying rather than skipping this gate.'
    exit 0
fi

if answer="$("$editor_bin" strict-gate)" && [ -n "$answer" ]; then
    printf '%s' "$answer"
else
    deny_closed 'editor-gate strict mode: ai-text-editor strict-gate gave no answer, so the installed client may be older than this plugin; re-run the installer. Denying rather than skipping this gate.'
fi
