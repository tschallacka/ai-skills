#!/usr/bin/env bash
# MODE: DEV
# test-installer-editor-steering.sh — an ai-text-editor install offers to turn
# down Claude Code's bash-first instruction, and writes only what was accepted.
#
# The instruction tells the agent to edit files with sed and heredocs rather
# than an editor, so while it is active the adapter this install just placed is
# usually bypassed. Two env settings turn it down; both are Claude Code's own,
# which is why a non-claude root is told so rather than edited.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-steer.XXXXXX")"
trap 'rm -rf "$work"' EXIT

install_into() { # <home> <skill> <target-subpath> [outfile]
    local home="$1" skill="$2" target="$3" out="${4:-$work/out}"
    # XDG_CONFIG_HOME is unset rather than merely HOME redirected: the worktree
    # grant resolves under it when set, so inheriting the developer's would
    # write a path outside the fixture.
    env -u XDG_CONFIG_HOME HOME="$home" "$BASH" "$installer" --skill "$skill" \
        --target "$home/$target" --yes >"$out" 2>&1 </dev/null
}

env_value() { # <cfg> <key>
    rjq -r --arg key "$2" '.env[$key] // "missing"' "$1"
}

seed_claude_home() { # <home> [settings-json]
    local home="$1" settings="${2:-{\}}"
    mkdir -p "$home/.claude"
    printf '%s\n' "$settings" >"$home/.claude/settings.json"
}

# ── the accepted offer writes the key, and only that key ────────────────────
home="$work/accept-home"
seed_claude_home "$home" '{ "model": "keep-me" }'
rc=0
install_into "$home" ai-text-editor .claude/skills || rc=$?
cfg="$home/.claude/settings.json"
t_assert_eq 'an ai-text-editor install completes' "$rc" '0'
t_assert_eq 'the step announced itself' \
    "$(awk '/== Step 4: ai-text-editor tool steering ==/ {found=1} END {print found+0}' "$work/out")" '1'
t_assert_eq 'the bash-first instruction is turned off' \
    "$(env_value "$cfg" CLAUDE_CODE_THRIFTY_SONIC)" 'false'
t_assert_eq 'the softer setting was not also written' \
    "$(env_value "$cfg" CLAUDE_CODE_COZY_TEAPOT)" 'missing'
t_assert_eq 'an unrelated pre-existing setting survived' \
    "$(rjq -r '.model' "$cfg")" 'keep-me'

# ── the warning names what declining costs ─────────────────────────────────
t_assert_eq 'the warning names an in-place rewrite exiting 0 on no match' \
    "$(awk '/exits 0 whether or not/ {found=1} END {print found+0}' "$work/out")" '1'
t_assert_eq 'the warning names both settings' \
    "$(awk '/CLAUDE_CODE_THRIFTY_SONIC=false/ {a=1} /CLAUDE_CODE_COZY_TEAPOT=relaxed/ {b=1} END {print a+0, b+0}' "$work/out")" '1 1'

# ── a second run reports it as present rather than writing again ───────────
rc=0
install_into "$home" ai-text-editor .claude/skills "$work/again" || rc=$?
t_assert_eq 'a second install completes' "$rc" '0'
t_assert_eq 'and reports the setting as already present' \
    "$(awk '/env.CLAUDE_CODE_THRIFTY_SONIC is already "false"/ {found=1} END {print found+0}' "$work/again")" '1'
t_assert_eq 'the value is unchanged' \
    "$(env_value "$cfg" CLAUDE_CODE_THRIFTY_SONIC)" 'false'

# ── a non-claude root is told the settings are not its own ─────────────────
home="$work/opencode-home"
seed_claude_home "$home"
mkdir -p "$home/.config/opencode"
rc=0
install_into "$home" ai-text-editor .config/opencode/skills || rc=$?
t_assert_eq 'an opencode-only install completes' "$rc" '0'
t_assert_eq 'the step says whose settings these are' \
    "$(awk "/No Claude Code root selected/ {found=1} END {print found+0}" "$work/out")" '1'
t_assert_eq 'and nothing was written to the claude settings' \
    "$(env_value "$home/.claude/settings.json" CLAUDE_CODE_THRIFTY_SONIC)" 'missing'

# ── an install without the skill never reaches the step ───────────────────
home="$work/planning-home"
seed_claude_home "$home"
rc=0
install_into "$home" planning .claude/skills || rc=$?
t_assert_eq 'a planning install completes' "$rc" '0'
t_assert_eq 'the steering step did not run' \
    "$(awk '/ai-text-editor tool steering/ {found=1} END {print found+0}' "$work/out")" '0'
t_assert_eq 'and no env block was added' \
    "$(env_value "$home/.claude/settings.json" CLAUDE_CODE_THRIFTY_SONIC)" 'missing'

t_end
