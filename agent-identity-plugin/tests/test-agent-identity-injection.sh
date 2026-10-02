#!/usr/bin/env bash
# MODE: DEV
# test-agent-identity-injection.sh -- the context-building logic
# hooks/subagent-start.sh depends on, tested directly against hooks/lib.sh
# rather than through rjq/stdin plumbing (same convention as
# tui-hint-plugin/tests/test-tui-hint-matching.sh).
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
plugin_dir="$(cd "$tests_dir/.." && pwd)"
repo_root="$(cd "$plugin_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

# shellcheck source=agent-identity-plugin/hooks/lib.sh
source "$plugin_dir/hooks/lib.sh"

context="$(agent_identity_context 'a4631ac048e789cbb' 'code-researcher')"

t_assert_eq 'a real agent_id/agent_type pair reports the agent id' \
    "$(printf '%s' "$context" | grep -c '^AGENT_ID=a4631ac048e789cbb')" '1'
t_assert_eq 'a real agent_id/agent_type pair reports the agent type' \
    "$(printf '%s' "$context" | grep -c 'AGENT_TYPE=code-researcher')" '1'
t_assert_eq 'the ai-text-editor instruction names the agent id as its argument' \
    "$(printf '%s' "$context" | grep -c 'agent: "a4631ac048e789cbb"')" '1'
t_assert_eq 'the interactive-shell instruction names the agent id as its flag' \
    "$(printf '%s' "$context" | grep -c -- '--agent a4631ac048e789cbb')" '1'
t_assert_eq 'chat is named as not yet supporting a per-call override' \
    "$(printf '%s' "$context" | grep -c 'chat-mcp does not yet support')" '1'

t_assert_eq 'an empty agent_id produces no context' \
    "$(agent_identity_context '' 'general-purpose' || true)" ''

t_expect_exit 1 'an empty agent_id returns non-zero' \
    agent_identity_context '' 'general-purpose'

missing_type="$(agent_identity_context 'solo-agent' '')"
t_assert_eq 'a missing agent_type falls back to "unknown" rather than an empty label' \
    "$(printf '%s' "$missing_type" | grep -c 'AGENT_TYPE=unknown')" '1'

t_end
