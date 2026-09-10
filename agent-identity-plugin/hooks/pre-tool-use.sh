#!/usr/bin/env bash
# MODE: PROD
# PreToolUse hook: the HARD half of T122/T123. Appends one line to a
# per-session register mapping this call's tool_use_id to the agent that
# made it, so an MCP server sharing one process across a parent and every
# subagent (measured: they are the same process, not one each) can resolve
# the real caller per call from the wire's own tool_use_id -- no argument
# for the model to pass, and nothing for it to get wrong or skip.
#
# Fires for every tool call, MCP or not; the cost of one is a single JSON
# parse and a line appended to a file already open for append, so it is not
# gated to MCP calls specifically.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=agent-identity-plugin/hooks/lib.sh
source "$script_dir/lib.sh"

payload="$(cat)"
session_id="$(printf '%s' "$payload" | rjq -r '.session_id // empty')"
tool_use_id="$(printf '%s' "$payload" | rjq -r '.tool_use_id // empty')"

# Nothing to join without both halves of the key; answer empty rather than
# fail the tool call over a register write this call does not need.
if [ -z "$session_id" ] || [ -z "$tool_use_id" ]; then
    printf '{}'
    exit 0
fi

agent_id="$(printf '%s' "$payload" | rjq -r '.agent_id // "main"')"
agent_type="$(printf '%s' "$payload" | rjq -r '.agent_type // "main"')"

dir="$(agent_identity_register_dir)"
mkdir -p "$dir"
register="$(agent_identity_register_path "$session_id")"

# Append, not rewrite: a register several calls are writing to at once must
# never see one call's line overwrite another's. A JSONL line under
# PIPE_BUF is one write() syscall in O_APPEND mode, so concurrent lines
# interleave whole, never mid-line.
rjq -n -c --arg tid "$tool_use_id" --arg aid "$agent_id" --arg atype "$agent_type" \
    '{tool_use_id: $tid, agent_id: $aid, agent_type: $atype}' >> "$register"

agent_identity_prune_stale

printf '{}'
