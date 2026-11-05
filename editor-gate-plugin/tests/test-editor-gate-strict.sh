#!/usr/bin/env bash
# MODE: DEV
# test-editor-gate-strict.sh -- the strict hook's own wiring: inert without the
# marker, and every way of failing to get an answer from the client is a deny.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
plugin_dir="$(cd "$tests_dir/.." && pwd)"
repo_root="$(cd "$plugin_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/editor-gate-strict.XXXXXX")"
trap 'rm -rf "$work"' EXIT

export XDG_CONFIG_HOME="$work/config"
shared_bin="$XDG_CONFIG_HOME/tsch-ai-skills/bin"
mkdir -p "$shared_bin"

# A copy, so the marker lands beside these hooks and not in the checkout.
installed="$work/editor-gate-plugin"
mkdir -p "$installed"
cp -R "$plugin_dir/hooks" "$installed/hooks"
strict_hook="$installed/hooks/pre-tool-use-strict.sh"
edit_hook="$installed/hooks/pre-tool-use-edit-write.sh"

write_call='{"tool_name":"Write","tool_input":{"file_path":"/x"}}'

write_client() { # <script body>
    printf '#!/bin/sh\n%s\n' "$1" >"$shared_bin/ai-text-editor"
    chmod +x "$shared_bin/ai-text-editor"
}

run_strict() {
    printf '%s' "$write_call" | "$BASH" "$strict_hook"
}

decision() { # <hook output>
    printf '%s' "$1" | jq -r '.hookSpecificOutput.permissionDecision // "none"'
}

write_client 'echo client-ran >&2; printf "%s" "{\"hookSpecificOutput\":{\"permissionDecision\":\"deny\"}}"'

t_assert_eq 'without the marker the hook answers empty' "$(run_strict 2>/dev/null)" '{}'
t_assert_eq 'without the marker the client never runs' "$(run_strict 2>&1 >/dev/null)" ''

touch "$installed/strict-mode"

t_assert_eq 'with the marker the client decides' "$(decision "$(run_strict 2>/dev/null)")" 'deny'

write_client 'cat >/dev/null; printf "{}"'
t_assert_eq 'an allowing client is passed through untouched' "$(run_strict)" '{}'

write_client 'exit 64'
t_assert_eq 'a client without the subcommand denies' "$(decision "$(run_strict 2>/dev/null)")" 'deny'

write_client 'exit 0'
t_assert_eq 'a client answering nothing denies' "$(decision "$(run_strict)")" 'deny'

rm -f "$shared_bin/ai-text-editor"
denied="$(run_strict)"
t_assert_eq 'a missing client denies' "$(decision "$denied")" 'deny'
t_assert_contains 'a missing client says what to do' 're-run the installer' "$denied"

t_assert_eq 'the Edit/Write reminder goes quiet under strict mode' \
    "$(printf '%s' "$write_call" | "$BASH" "$edit_hook")" '{}'

t_end
