#!/usr/bin/env bash
# MODE: PROD
# SubagentStart hook: injects the subagent's own id/type into its context.
#
# This is the SOFT half of T122/T123's design (see ../docs/README.md): the
# subagent now knows who it is well enough to name itself explicitly on a
# CLI call it makes with its own shell (ai-text-editor --session, or
# CHAT_SESSION_ID), but nothing enforces that it actually does. An MCP call
# needs none of this -- see pre-tool-use.sh, the HARD half, for that path.
#
# Measured 2026-09-08 (recorded in TODO T122): the hook payload's agent_id
# matches, character for character, the id the Agent tool returned to the
# spawner, and additionalContext measurably reaches the subagent's own
# context before it calls anything.
set -euo pipefail

payload="$(cat)"
agent_id="$(printf '%s' "$payload" | rjq -r '.agent_id // "main"')"
agent_type="$(printf '%s' "$payload" | rjq -r '.agent_type // "main"')"

context="You are agent $agent_id (type: $agent_type). Pass this exact id explicitly on every CLI call you make yourself to a session-aware tool: ai-text-editor's --session $agent_id, or CHAT_SESSION_ID=$agent_id for chat. An MCP tool call needs none of this -- the server resolves you from this hook without your having to say anything."

rjq -n -c --arg ctx "$context" '{"hookSpecificOutput":{"hookEventName":"SubagentStart","additionalContext":$ctx}}'
