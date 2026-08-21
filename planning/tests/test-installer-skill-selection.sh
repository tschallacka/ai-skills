#!/usr/bin/env bash
# MODE: DEV
# test-installer-skill-selection.sh — how --skill resolves to a set of skills.
#
# `--skill a --skill b` used to keep only b and install something other than what
# was asked, with no warning. It was documented behaviour, so this file pins the
# replacement rather than the old rule: the flag accumulates, and the comma form
# and the repeated form reach the same answer.
#
# Asserted against what the installer actually installed, not only its "Selected
# skills:" line: the line is a claim and the directories are the outcome.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-selection.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The installed directory names, sorted, for one invocation.
installed() { # <args...>
    local target
    target="$(mktemp -d "$work/t.XXXXXX")"
    ( "$BASH" "$installer" "$@" --target "$target" --yes ) >/dev/null 2>&1 || true
    ( cd "$target" && find . -mindepth 1 -maxdepth 1 -type d | sed 's|^\./||' | LC_ALL=C sort | tr '\n' ' ' )
}

# ── the repeated form accumulates ───────────────────────────────────────────
t_assert_eq 'two --skill flags install both' \
    "$(installed --skill todo --skill bug-report)" 'bug-report todo '
t_assert_eq 'and order does not change the set' \
    "$(installed --skill bug-report --skill todo)" 'bug-report todo '
t_assert_eq 'three flags install three' \
    "$(installed --skill todo --skill bug-report --skill brainstorm)" 'brainstorm bug-report todo '

# ── it agrees with the comma form, which is what it is joined into ───────────
t_assert_eq 'the comma form gives the same set' \
    "$(installed --skill todo,bug-report)" "$(installed --skill todo --skill bug-report)"
t_assert_eq 'mixing the two spellings works' \
    "$(installed --skill todo,brainstorm --skill bug-report)" 'brainstorm bug-report todo '

# ── de-duplication, and the menu-number spelling ────────────────────────────
t_assert_eq 'a repeated skill is installed once' \
    "$(installed --skill todo --skill todo)" 'todo '
t_assert_eq 'a menu number and a name combine' \
    "$(installed --skill 7 --skill todo)" 'bug-report todo '

# ── all, wherever it appears ────────────────────────────────────────────────
# `--skill all --skill todo` is not a contradiction to resolve by ordering.
every="$("$BASH" -c 'source "'"$repo_root"'/installer/src/05-config.sh"; printf "%s\n" "${SKILL_NAMES[@]}" | LC_ALL=C sort | tr "\n" " "')"
t_assert_eq 'all installs every skill' "$(installed --skill all)" "$every"
t_assert_eq 'all combined with a name still installs every skill' \
    "$(installed --skill all --skill todo)" "$every"
# The bare 6 keeps meaning all, whole-string only: with seven skills 6 is also a
# position, and reading it as all inside a list would install one thing when a
# list was asked for.
t_assert_eq 'the bare menu answer 6 still means all' "$(installed --skill 6)" "$every"

# ── the refusals are unchanged ───────────────────────────────────────────────
rc=0
"$BASH" "$installer" --skill nosuchskill --target "$work/none" --yes >/dev/null 2>&1 || rc=$?
t_assert_eq 'an unknown name is still refused' "$([ "$rc" -ne 0 ] && printf refused)" 'refused'
rc=0
"$BASH" "$installer" --skill todo --skill nosuchskill --target "$work/none2" --yes >/dev/null 2>&1 || rc=$?
t_assert_eq 'an unknown name in a later flag is refused too' \
    "$([ "$rc" -ne 0 ] && printf refused)" 'refused'
rc=0
"$BASH" "$installer" --skill --target "$work/none3" --yes >/dev/null 2>&1 || rc=$?
t_assert_eq 'a --skill with no value is refused' "$([ "$rc" -ne 0 ] && printf refused)" 'refused'

t_end
