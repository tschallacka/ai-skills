#!/usr/bin/env bash
# MODE: DEV
# test-installer-opencode-permissions.sh — the opencode permission autoset edits
# the config that actually exists, and never rewrites one it cannot parse.
#
# opencode reads ~/.config/opencode/opencode.json or opencode.jsonc, either
# name, JSON-C syntax allowed in both. The editor used to look at opencode.json
# only, so on a machine whose config is the .jsonc one (or that has none yet)
# the grant silently did nothing -- and a commented config would have been
# rebuilt from {} through rjq, stripping every comment in it.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-oxperm.XXXXXX")"
trap 'rm -rf "$work"' EXIT

install_planning() { # <home>
    local home="$1"
    mkdir -p "$home/.config/opencode"
    # XDG_CONFIG_HOME is unset, not merely HOME redirected: the plans directory
    # the grant names resolves under it when set, so inheriting the developer's
    # would grant a path outside the fixture.
    env -u XDG_CONFIG_HOME HOME="$home" "$BASH" "$installer" --skill planning \
        --target "$home/.config/opencode/skills" --yes \
        >"$work/out" 2>&1 </dev/null
}

grant_state() { # <cfg> <tool> <glob>
    rjq -r --arg tool "$2" --arg glob "$3" '.permission[$tool][$glob] // "missing"' "$1"
}

# ── an existing opencode.jsonc is the one edited ────────────────────────────
home="$work/jsonc-home"
mkdir -p "$home/.config/opencode"
printf '{\n  "$schema": "https://opencode.ai/config.json",\n  "model": "opencode/big-pickle"\n}\n' \
    >"$home/.config/opencode/opencode.jsonc"
rc=0
install_planning "$home" || rc=$?
t_assert_eq 'a jsonc-only config installs cleanly' "$rc" '0'
t_assert_eq 'the plans grant landed in the jsonc file' \
    "$(grant_state "$home/.config/opencode/opencode.jsonc" read "$home/.config/tsch-ai-skills/plans/**")" 'allow'
t_assert_eq 'the pre-existing model field survived' \
    "$(rjq -r '.model' "$home/.config/opencode/opencode.jsonc")" 'opencode/big-pickle'
t_assert_eq 'and no parallel opencode.json appeared' \
    "$([ -f "$home/.config/opencode/opencode.json" ] && printf yes || printf no)" 'no'

# ── no config at all: one is created holding the grant ──────────────────────
home="$work/fresh-home"
rc=0
install_planning "$home" || rc=$?
t_assert_eq 'a machine with no config installs cleanly' "$rc" '0'
cfg="$home/.config/opencode/opencode.json"
t_assert_eq 'an opencode.json was created' "$([ -f "$cfg" ] && printf yes || printf no)" 'yes'
t_assert_eq 'carrying the schema reference' \
    "$(rjq -r '."$schema"' "$cfg")" 'https://opencode.ai/config.json'
t_assert_eq 'and a bash grant scoped to the installed scripts' \
    "$(grant_state "$cfg" "bash" "$home/.config/opencode/skills/planning/scripts/**")" 'allow'

# ── a commented jsonc is refused, not rewritten ─────────────────────────────
home="$work/comments-home"
mkdir -p "$home/.config/opencode"
cat >"$home/.config/opencode/opencode.jsonc" <<'EOF'
{
  // chosen deliberately
  "$schema": "https://opencode.ai/config.json",
}
EOF
cp "$home/.config/opencode/opencode.jsonc" "$work/commented-before"
rc=0
install_planning "$home" || rc=$?
t_assert_eq 'a commented config still installs cleanly' "$rc" '0'
t_assert_eq 'the commented config was not touched' \
    "$(cmp -s "$work/commented-before" "$home/.config/opencode/opencode.jsonc" && printf same || printf changed)" 'same'
t_assert_eq 'with manual instructions instead of a rewrite' \
    "$(grep -c 'is not strict JSON' "$work/out" || true)" '1'

# ── an existing opencode.json keeps user rules and stays idempotent ─────────
home="$work/merge-home"
mkdir -p "$home/.config/opencode"
printf '{\n  "$schema": "https://opencode.ai/config.json",\n  "permission": {"bash": {"git *": "allow"}}\n}\n' \
    >"$home/.config/opencode/opencode.json"
rc=0
install_planning "$home" || rc=$?
t_assert_eq 'an existing opencode.json installs cleanly' "$rc" '0'
t_assert_eq 'the user rule survived the merge' \
    "$(grant_state "$home/.config/opencode/opencode.json" bash 'git *')" 'allow'
before="$(rjq '.permission.read | length' "$home/.config/opencode/opencode.json")"
rc=0
install_planning "$home" || rc=$?
after="$(rjq '.permission.read | length' "$home/.config/opencode/opencode.json")"
t_assert_eq 'a second run reports nothing to add' \
    "$(grep -c 'permissions already present' "$work/out" || true)" '1'
t_assert_eq 'and does not duplicate entries' "$( [ "$before" -eq "$after" ] && printf same || printf grew)" 'same'

t_end
