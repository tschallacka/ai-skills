#!/usr/bin/env bash
# MODE: PROD
# PreToolUse hook, Edit|Write only: a SOFT reminder, never blocking, that the
# ai-text-editor MCP/skill is usually the better instrument even for Claude
# Code's own native Edit/Write tools. Leads with the gap Edit's own
# uniqueness check does NOT cover -- whether the file changed on disk since
# it was last read, not just whether the match text is unambiguous -- rather
# than only listing journaling/logging as added features. A live-steered
# session read an earlier, feature-listing version of this reminder and
# reasoned "Edit's uniqueness check already protects me," only recognizing
# the actual gap when asked directly afterward (see
# benchmark/ai-text-editor-usage/FINDINGS.md); this wording leads with the
# distinction that session had to be asked to find. additionalContext only;
# permissionDecision is always "allow".
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=editor-gate-plugin/hooks/lib.sh
source "$script_dir/lib.sh"

rjq_bin="$(editor_gate_rjq_bin)" || { printf '{}'; exit 0; }
payload="$(cat)"
tool_name="$(printf '%s' "$payload" | "$rjq_bin" -r '.tool_name // empty')"

case "$tool_name" in
    Edit | Write) ;;
    *) printf '{}'; exit 0 ;;
esac

reminder="$(editor_gate_reminder EDITOR_GATE_EDIT_WRITE_TEXT "This edit could also go through the ai-text-editor MCP/skill. Edit's own match check only confirms the string is unique in what you already read -- it does not re-check the file against disk, so a change landing between your read and this write (another process, a formatter, a teammate) gets silently edited over. ai-text-editor's expected_text and revision guard catch exactly that case and refuse instead; its journal also survives a git checkout that would discard this edit. Not required here; just worth knowing. Minimal schema for the two calls this needs, so switching costs no extra ToolSearch round trip -- open: {\"file\":{\"type\":\"string\",\"description\":\"path; opens it and returns tab_id+revision if not already open\"}}; replace (needs tab_id or file, plus the revision open/read last returned): {\"range_start_line\":{\"type\":\"integer\",\"description\":\"inclusive first line\"},\"range_end_line\":{\"type\":\"integer\",\"description\":\"inclusive last line; its newline goes with it, so replacing with no text deletes the lines outright\"},\"range_start_byte\":{\"type\":\"integer\"},\"range_end_byte\":{\"type\":\"integer\",\"description\":\"half-open byte span, exactly a search hit's byte_start/byte_end\"},\"match_id\":{\"type\":\"string\",\"description\":\"a search hit's own id; resolves to its exact span, no expected_text needed\"},\"expected_text\":{\"type\":\"string\",\"description\":\"verified BEFORE anything is deleted, refused by name on mismatch\"},\"text\":{\"type\":\"string\",\"description\":\"replacement text\"}}. (Trimmed: full schema also has insert/move/copy/bulk_edit and replace's offset/delete_len/cursor_id/bytes_base64/symbol/range_start_match addressing -- ToolSearch mcp__ai-text-editor__replace for those.)")"
if [ -z "$reminder" ]; then
    printf '{}'
    exit 0
fi
editor_gate_emit_context "$rjq_bin" "$reminder"
