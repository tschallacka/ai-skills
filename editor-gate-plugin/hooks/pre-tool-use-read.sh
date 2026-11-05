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

reminder="$(editor_gate_reminder EDITOR_GATE_READ_TEXT "Before reading the whole file: ai-text-editor's search (exact, regex, or fuzzy) can often find what you need directly without pulling the whole file into context, and a hit's own byte_start/byte_end addresses a later replace --range-start-byte/--range-end-byte directly -- no need to re-quote the matched text as an old_string the way Edit requires. Not required here; just worth knowing. Minimal schema (no extra ToolSearch round trip): search: {\"mode\":{\"type\":\"string\",\"description\":\"exact_text, exact_bytes, wildcard, shell_wildcard, path_wildcard, regex_rust, regex_pcre2, fuzzy_edit, fuzzy_subsequence, fuzzy_token, fuzzy_ngram, fuzzy_phonetic, fuzzy_soundex\"},\"query\":{\"type\":\"string\",\"description\":\"search query, interpreted by mode\"},\"range_start_line\":{\"type\":\"integer\",\"description\":\"inclusive first line, required on large tabs\"},\"range_end_line\":{\"type\":\"integer\",\"description\":\"inclusive last line, required on large tabs\"}} (required: mode). Each hit reports byte_start/byte_end, which feed replace's range_start_byte/range_end_byte directly. (Trimmed: full schema also has query_base64/limit/order/gradient/preview_lines/range_start_byte/range_end_byte -- ToolSearch mcp__ai-text-editor__search for those.)")"
if [ -z "$reminder" ]; then
    printf '{}'
    exit 0
fi
editor_gate_emit_context "$rjq_bin" "$reminder"
