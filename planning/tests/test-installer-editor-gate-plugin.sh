#!/usr/bin/env bash
# MODE: DEV
# test-installer-editor-gate-plugin.sh — an ai-text-editor install places
# editor-gate-plugin/ under a selected Claude Code root (Bash/Edit/Write
# PreToolUse hooks are Claude Code's own, same gate editor_steering_step
# already uses), and skips it entirely for a non-Claude root.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-editor-gate.XXXXXX")"
trap 'rm -rf "$work"' EXIT

install_into() { # <home> <target-subpath> [outfile]
    local home="$1" target="$2" out="${3:-$work/out}"
    env -u XDG_CONFIG_HOME HOME="$home" "$BASH" "$installer" --skill ai-text-editor \
        --target "$home/$target" --yes >"$out" 2>&1 </dev/null
}

# ── Claude Code: the plugin directory lands under the selected root ────────
home="$work/claude-home"
rc=0
install_into "$home" .claude/skills || rc=$?
t_assert_eq 'a Claude Code ai-text-editor install completes' "$rc" '0'

plugin_dir="$home/.claude/skills/editor-gate-plugin"
t_assert_eq 'plugin.json landed' \
    "$([ -f "$plugin_dir/.claude-plugin/plugin.json" ] && printf yes || printf no)" 'yes'
t_assert_eq 'hooks.json landed' \
    "$([ -f "$plugin_dir/hooks/hooks.json" ] && printf yes || printf no)" 'yes'
t_assert_eq 'pre-tool-use-bash.sh landed and is executable' \
    "$([ -x "$plugin_dir/hooks/pre-tool-use-bash.sh" ] && printf yes || printf no)" 'yes'
t_assert_eq 'pre-tool-use-edit-write.sh landed and is executable' \
    "$([ -x "$plugin_dir/hooks/pre-tool-use-edit-write.sh" ] && printf yes || printf no)" 'yes'
t_assert_eq 'editor-token landed and is executable' \
    "$([ -x "$plugin_dir/hooks/editor-token" ] && printf yes || printf no)" 'yes'
t_assert_eq 'lib.sh landed and is executable' \
    "$([ -x "$plugin_dir/hooks/lib.sh" ] && printf yes || printf no)" 'yes'
t_assert_eq 'README.md is not copied (the plugin loader never reads it)' \
    "$([ -f "$plugin_dir/README.md" ] && printf yes || printf no)" 'no'

# ── a non-Claude root gets nothing: nothing here is that harness's own ─────
home="$work/opencode-home"
rc=0
install_into "$home" .config/opencode/skills || rc=$?
t_assert_eq 'an opencode ai-text-editor install completes' "$rc" '0'
t_assert_eq 'no editor-gate-plugin directory is created for a non-Claude root' \
    "$([ -d "$home/.config/opencode/skills/editor-gate-plugin" ] && printf yes || printf no)" 'no'

t_end
