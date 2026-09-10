#!/usr/bin/env bash
# MODE: DEV
# test-installer-tui-hint-plugin.sh — an interactive-shell install places the
# tui-hint-plugin per selected root: a Claude Code root gets its own copy of
# the plugin directory (the loader reads it from there); opencode gets the
# .js file copied to a stable location and registered once in opencode.jsonc's
# "plugin" array, regardless of how many opencode roots were selected.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-tui-hint.XXXXXX")"
trap 'rm -rf "$work"' EXIT

install_into() { # <home> <target-subpath> [outfile]
    local home="$1" target="$2" out="${3:-$work/out}"
    env -u XDG_CONFIG_HOME HOME="$home" "$BASH" "$installer" --skill interactive-shell \
        --target "$home/$target" --yes >"$out" 2>&1 </dev/null
}

# ── Claude Code: the plugin directory lands under the selected root ────────
home="$work/claude-home"
rc=0
install_into "$home" .claude/skills || rc=$?
t_assert_eq 'a Claude Code interactive-shell install completes' "$rc" '0'

plugin_dir="$home/.claude/skills/tui-hint-plugin"
t_assert_eq 'plugin.json landed' \
    "$([ -f "$plugin_dir/.claude-plugin/plugin.json" ] && printf yes || printf no)" 'yes'
t_assert_eq 'hooks.json landed' \
    "$([ -f "$plugin_dir/hooks/hooks.json" ] && printf yes || printf no)" 'yes'
t_assert_eq 'pre-tool-use.sh landed and is executable' \
    "$([ -x "$plugin_dir/hooks/pre-tool-use.sh" ] && printf yes || printf no)" 'yes'
t_assert_eq 'lib.sh landed and is executable' \
    "$([ -x "$plugin_dir/hooks/lib.sh" ] && printf yes || printf no)" 'yes'
t_assert_eq 'README.md is not copied (the plugin loader never reads it)' \
    "$([ -f "$plugin_dir/README.md" ] && printf yes || printf no)" 'no'

# ── opencode: one copy of the .js, registered once in opencode.jsonc ───────
home="$work/opencode-home"
rc=0
install_into "$home" .config/opencode/skills || rc=$?
t_assert_eq 'an opencode interactive-shell install completes' "$rc" '0'

destination="$home/.config/tsch-ai-skills/tui-hint-plugin/tui-hint-plugin.js"
t_assert_eq 'the plugin .js landed at the stable location' \
    "$([ -f "$destination" ] && printf yes || printf no)" 'yes'

cfg="$home/.config/opencode/opencode.json"
t_assert_eq 'opencode.json was created' \
    "$([ -f "$cfg" ] && printf yes || printf no)" 'yes'
t_assert_eq 'the plugin path was added to the plugin array' \
    "$(rjq -r --arg entry "$destination" '[(.plugin // [])[] | select(. == $entry)] | length' "$cfg")" '1'

# ── a second run is idempotent, not a second identical entry ───────────────
rc=0
install_into "$home" .config/opencode/skills "$work/again" || rc=$?
t_assert_eq 'a second install completes' "$rc" '0'
t_assert_eq 'the plugin array still has exactly one matching entry' \
    "$(rjq -r --arg entry "$destination" '[(.plugin // [])[] | select(. == $entry)] | length' "$cfg")" '1'

t_end
