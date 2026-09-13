#!/usr/bin/env bash
# MODE: PROD
# SubagentStart hook: injects the subagent's own agent_id/agent_type into
# its context as additionalContext, and instructs it to declare that id on
# every ai-text-editor and interactive-shell call. Measured 2026-09-08
# (T122's own note): a subagent's environment is byte-identical to its
# parent's, so nothing downstream of the shell can tell them apart on its
# own -- this hook is the only place the harness hands the id over, and it
# is soft (context, not an environment variable a child process inherits),
# which is why it pairs with each tool's own refusal/scoping rather than
# replacing it. Never blocks or modifies anything; this only annotates.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=agent-identity-plugin/hooks/lib.sh
source "$script_dir/lib.sh"

rjq_bin="$(agent_identity_rjq_bin)" || { printf '{}'; exit 0; }
payload="$(cat)"
agent_id="$(printf '%s' "$payload" | "$rjq_bin" -r '.agent_id // empty')"
agent_type="$(printf '%s' "$payload" | "$rjq_bin" -r '.agent_type // empty')"

context="$(agent_identity_context "$agent_id" "$agent_type")" || { printf '{}'; exit 0; }

"$rjq_bin" -n -c --arg context "$context" '
{
  hookSpecificOutput: {
    hookEventName: "SubagentStart",
    additionalContext: $context
  }
}'
