#!/usr/bin/env bash
# MODE: PROD
# PreToolUse hook, Read only: a SOFT reminder, never blocking, that
# ai-text-editor's search can often answer what a Read is about to spend
# context pulling in whole, and that a search hit's own byte range addresses
# a later replace directly -- no need to re-quote the matched text as Edit's
# old_string requires. additionalContext only; permissionDecision is always
# "allow".
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=editor-gate-plugin/hooks/lib.sh
source "$script_dir/lib.sh"

rjq_bin="$(editor_gate_rjq_bin)" || { printf '{}'; exit 0; }
payload="$(cat)"
tool_name="$(printf '%s' "$payload" | "$rjq_bin" -r '.tool_name // empty')"

case "$tool_name" in
    Read) ;;
    *) printf '{}'; exit 0 ;;
esac

editor_gate_emit_context "$rjq_bin" "Before reading the whole file: ai-text-editor's search (exact, regex, or fuzzy) can often find what you need directly without pulling the whole file into context, and a hit's own byte_start/byte_end addresses a later replace --range-start-byte/--range-end-byte directly -- no need to re-quote the matched text as an old_string the way Edit requires. Not required here; just worth knowing."
