#!/usr/bin/env bash
# MODE: PROD
# PreToolUse hook, Edit|Write only: a SOFT reminder, never blocking, that the
# ai-text-editor MCP/skill is usually the better instrument even for Claude
# Code's own native Edit/Write tools -- it journals every change (survives a
# git checkout that would discard an Edit/Write's result) and logs what
# changed, on top of expected_text's mismatch-refusal. additionalContext
# only; permissionDecision is always "allow".
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=editor-gate-plugin/hooks/lib.sh
source "$script_dir/lib.sh"

payload="$(cat)"
tool_name="$(editor_gate_json_field tool_name <<<"$payload" || true)"

case "$tool_name" in
    Edit | Write) ;;
    *) printf '{}'; exit 0 ;;
esac

printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","additionalContext":"This edit could also go through the ai-text-editor MCP/skill -- same result, plus two things this tool does not give you: every change is journaled (recoverable even after a git checkout discards the working tree) and logged, and expected_text refuses on a mismatch instead of editing the wrong span silently. Not required here; just worth knowing."}}'
