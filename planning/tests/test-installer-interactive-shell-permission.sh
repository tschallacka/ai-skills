#!/usr/bin/env bash
# MODE: DEV
# test-installer-interactive-shell-permission.sh — an interactive-shell install
# grants execution of the wrapper, scoped to the binaries it just placed.
#
# A denied Bash call does not read as "ask for permission" to an agent; it reads
# as "this tool does not work", after which the agent reaches for a headless
# command that cannot observe a terminal program at all. The grant is what keeps
# the skill reachable, so it is offered on every install that places it.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-ishperm.XXXXXX")"
trap 'rm -rf "$work"' EXIT

install_into() { # <home> <skill> <target-subpath> [outfile]
    local home="$1" skill="$2" target="$3" out="${4:-$work/out}"
    # XDG_CONFIG_HOME is unset rather than merely HOME redirected: the worktree
    # grant resolves under it when set, so inheriting the developer's would
    # write a path outside the fixture.
    env -u XDG_CONFIG_HOME HOME="$home" "$BASH" "$installer" --skill "$skill" \
        --target "$home/$target" --yes >"$out" 2>&1 </dev/null
}

# Counts the allow entries equal to the wrapper grant, so a duplicate on a
# second run is a failure rather than a pass.
grant_count() { # <cfg> <bins>
    rjq -r --arg rule "Bash($2/**:*)" \
        '[(.permissions.allow // [])[] | select(. == $rule)] | length' "$1"
}

home="$work/claude-home"
mkdir -p "$home/.claude"
printf '{\n  "model": "keep-me"\n}\n' >"$home/.claude/settings.json"
cfg="$home/.claude/settings.json"
bins="$home/.claude/skills/interactive-shell/bin"

rc=0
install_into "$home" interactive-shell .claude/skills || rc=$?
t_assert_eq 'an interactive-shell install completes' "$rc" '0'
t_assert_eq 'the step announced itself' \
    "$(awk '/== Step 3: interactive-shell execution permission ==/ {found=1} END {print found+0}' "$work/out")" '1'
t_assert_eq 'the wrapper grant landed, scoped to the installed binaries' \
    "$(grant_count "$cfg" "$bins")" '1'
t_assert_eq 'an unrelated pre-existing setting survived' \
    "$(rjq -r '.model' "$cfg")" 'keep-me'
t_assert_eq 'the grant does not cover the whole skill directory' \
    "$(rjq -r '[(.permissions.allow // [])[] | select(. == "Bash('"$home"'/.claude/skills/interactive-shell/**:*)")] | length' "$cfg")" '0'

# ── a second run is idempotent, not a second identical rule ────────────────
rc=0
install_into "$home" interactive-shell .claude/skills "$work/again" || rc=$?
t_assert_eq 'a second install completes' "$rc" '0'
t_assert_eq 'and reports the grant as already in place' \
    "$(awk '/interactive-shell grant already in place/ {found=1} END {print found+0}' "$work/again")" '1'
t_assert_eq 'leaving exactly one copy of the rule' \
    "$(grant_count "$cfg" "$bins")" '1'

# ── an install without the skill never reaches the step ───────────────────
home="$work/planning-home"
mkdir -p "$home/.claude"
printf '{}\n' >"$home/.claude/settings.json"
rc=0
install_into "$home" planning .claude/skills || rc=$?
t_assert_eq 'a planning install completes' "$rc" '0'
t_assert_eq 'the wrapper step did not run' \
    "$(awk '/interactive-shell execution permission/ {found=1} END {print found+0}' "$work/out")" '0'
t_assert_eq 'and no wrapper grant was added' \
    "$(grant_count "$home/.claude/settings.json" "$home/.claude/skills/interactive-shell/bin")" '0'

t_end
