#!/usr/bin/env bash
# MODE: PROD
# PreToolUse hook, Bash only: blocks a command that rewrites a file in place
# with sed -i, perl -i, or a heredoc-fed write (script body or plain shell)
# unless it carries a minted, single-use token (hooks/editor-token) bound to
# that exact command. The ai-text-editor MCP/skill is the better instrument
# for editing a file: it verifies what it replaces (expected_text refuses on
# a mismatch instead of silently doing nothing, the way `sed -i` exits 0 on
# a typo'd pattern) and journals every change, which survives a git checkout
# that discards a shell rewrite. Deny only -- this never rewrites the
# command itself.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=editor-gate-plugin/hooks/lib.sh
source "$script_dir/lib.sh"

MESSAGE='In-place shell editing is gated -- the ai-text-editor MCP/skill is the better tool.

Before retrying, ask which of these you are doing:

  1. EDITING A FILE. Prefer the ai-text-editor skill or its MCP tool, in
     whichever form this session has:
       mcp__ai-text-editor__open      -> gives you a tab_id and a revision
       mcp__ai-text-editor__replace   -> range_start_line/range_end_line, plus
                                         expected_text so a mismatch REFUSES
                                         instead of silently doing nothing
       mcp__ai-text-editor__save      -> carries the revision guard
     No MCP registered here? Invoke the ai-text-editor skill directly instead.

  2. GENERATING a file wholesale, where no existing content is at risk -- a
     scratch fixture, a probe script, a temp file under $TMPDIR. Use the
     Write tool, or mint a token if it must be a shell redirect.

  3. A MECHANICAL SWEEP the editor genuinely cannot express -- the same
     change across dozens of files. That is a real case, and it is what the
     token is for. Say so when you mint it.

     ${CLAUDE_PLUGIN_ROOT}/hooks/editor-token \
       --why '"'"'<why the editor cannot do this one>'"'"' \
       --command '"'"'<the exact command you are about to run>'"'"'

     It prints EDIT_OK=<token>; prefix the command with that. The token
     works once, expires in 120 seconds, and is bound to that exact command.

Why this exists: sed -i rewrites and exits 0 whether or not the pattern
matched, so a typo looks like success. A heredoc stacks the shell'"'"'s escaping
on top of the target file'"'"'s own syntax. Neither verifies what it replaces,
while the editor'"'"'s expected_text refuses on mismatch and its journal
survives a git checkout that had discarded a shell rewrite.'

payload="$(cat)"
tool_name="$(editor_gate_json_field tool_name <<<"$payload" || true)"
command_line="$(editor_gate_json_field command <<<"$payload" || true)"

if [ "$tool_name" != "Bash" ] || [ -z "$command_line" ]; then
    printf '{}'
    exit 0
fi

if ! editor_gate_matches "$command_line"; then
    printf '{}'
    exit 0
fi

if token="$(printf '%s' "$command_line" | grep -Eo 'EDIT_OK=[0-9a-f]{32}' | head -n1 | cut -d= -f2)"; [ -n "$token" ]; then
    # The token authorises the command it was minted for, which never
    # itself contained "EDIT_OK=..." -- strip that exact prefix (any run of
    # whitespace around it) before comparing, or a token would only ever
    # match a stored command that also happened to carry the token.
    stripped_line="$(printf '%s' "$command_line" | sed -E 's/(^|[[:space:]])EDIT_OK=[0-9a-f]{32}([[:space:]]|$)/ /g')"
    if reason="$(editor_gate_consume "$token" "$stripped_line")"; then
        printf '{}'
        exit 0
    fi
    escaped="$(editor_gate_json_escape "Token rejected: $reason
$MESSAGE")"
    printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"%s"}}' "$escaped"
    exit 0
fi

escaped="$(editor_gate_json_escape "$MESSAGE")"
printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"%s"}}' "$escaped"
