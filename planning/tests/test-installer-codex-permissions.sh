#!/usr/bin/env bash
# MODE: DEV
# test-installer-codex-permissions.sh — B235: the installer edits codex's
# config.toml the way it edits Claude Code's and opencode's config.
#
# codex has no JSON permission file, so rjq (the installer's only permitted
# runtime dependency) cannot edit it directly. This exercises the one
# well-defined TOML shape the installer is willing to touch -- a single-line
# `writable_roots = [...]` array -- and confirms it refuses, rather than
# risking a bad edit, on anything more complex.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-codexperm.XXXXXX")"
trap 'rm -rf "$work"' EXIT

install_planning() { # <home>
    local home="$1"
    mkdir -p "$home/.codex"
    env -u XDG_CONFIG_HOME HOME="$home" "$BASH" "$installer" --skill planning \
        --target "$home/.codex/skills" --yes \
        >"$work/out" 2>&1 </dev/null
}

# ── no config.toml at all: one is created holding the grant ────────────────
home="$work/fresh-home"
rc=0
install_planning "$home" || rc=$?
t_assert_eq 'a machine with no config.toml installs cleanly' "$rc" '0'
cfg="$home/.codex/config.toml"
t_assert_eq 'a config.toml was created' "$([ -f "$cfg" ] && printf yes || printf no)" 'yes'
t_assert_eq 'naming the plans directory' \
    "$(grep -c "$home/.config/tsch-ai-skills/plans" "$cfg")" '1'
t_assert_eq 'and the installed scripts directory' \
    "$(grep -c "$home/.codex/skills/planning/scripts" "$cfg")" '1'

# ── an existing config.toml with unrelated content keeps it, grant prepended ──
home="$work/prepend-home"
mkdir -p "$home/.codex"
printf 'model = "gpt-5"\n\n[some_other_table]\nkey = "value"\n' >"$home/.codex/config.toml"
rc=0
install_planning "$home" || rc=$?
t_assert_eq 'an existing config.toml installs cleanly' "$rc" '0'
cfg="$home/.codex/config.toml"
t_assert_eq 'the pre-existing model field survived' \
    "$(grep -c 'model = "gpt-5"' "$cfg")" '1'
t_assert_eq 'the pre-existing table survived' \
    "$(grep -c '\[some_other_table\]' "$cfg")" '1'
t_assert_eq 'the writable_roots line was prepended before it, not appended after' \
    "$(head -1 "$cfg" | grep -c 'writable_roots')" '1'

# ── an existing single-line array is merged into, idempotently ─────────────
home="$work/merge-home"
mkdir -p "$home/.codex"
printf 'sandbox_workspace_write.writable_roots = ["/tmp", "/keep"]\n' >"$home/.codex/config.toml"
rc=0
install_planning "$home" || rc=$?
t_assert_eq 'a config.toml with an existing array installs cleanly' "$rc" '0'
cfg="$home/.codex/config.toml"
t_assert_eq 'the pre-existing entries survived' "$(grep -c '"/keep"' "$cfg")" '1'
t_assert_eq 'the plans directory was added' \
    "$(grep -c "$home/.config/tsch-ai-skills/plans" "$cfg")" '1'
t_assert_eq 'still exactly one writable_roots line, not a second one appended' \
    "$(grep -c 'writable_roots' "$cfg")" '1'
# Re-running must not duplicate what is already there.
rc=0
install_planning "$home" || rc=$?
t_assert_eq 'a second install run is idempotent' "$rc" '0'
t_assert_eq 'the plans entry was not duplicated' \
    "$(grep -o "$home/.config/tsch-ai-skills/plans" "$cfg" | wc -l | tr -d ' ')" '1'

# ── a multi-line array is refused, not risked ───────────────────────────────
home="$work/multiline-home"
mkdir -p "$home/.codex"
printf 'sandbox_workspace_write.writable_roots = [\n  "/tmp",\n]\n' >"$home/.codex/config.toml"
cp "$home/.codex/config.toml" "$work/multiline-before"
rc=0
install_planning "$home" || rc=$?
t_assert_eq 'a multi-line array config still installs cleanly' "$rc" '0'
t_assert_eq 'the multi-line config was not touched' \
    "$(cmp -s "$work/multiline-before" "$home/.codex/config.toml" && printf same || printf changed)" 'same'
# Once for the planning grant, once for the worktrees grant -- both run on
# every install and both correctly refuse the same file.
t_assert_eq 'with manual instructions instead of a risky edit' \
    "$(grep -c 'is not a single-line array' "$work/out" || true)" '2'

t_end
