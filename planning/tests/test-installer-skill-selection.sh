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
# No cleanup trap of our own: $work sits under $TMPDIR, which lib-test.sh
# already rewrote to $T_TMPDIR and already owns via t_tmpdir_cleanup (set as
# an EXIT trap when it was sourced above). A second `trap ... EXIT` here does
# not chain with that one, it REPLACES it -- so this file used to delete its
# own install.sh logs before t_tmpdir_cleanup's on-failure evidence dump ever
# ran, leaving a real "all installs every skill" regression with no clue why
# in the CI log beyond the expected/got mismatch. lib-test.sh's own cleanup
# removes $work as part of $T_TMPDIR regardless; nothing here needs to repeat it.

# The installed directory names, sorted, for one invocation. install.sh's own
# stdout/stderr goes to a log beside $target rather than /dev/null, so a
# future failure's evidence dump (t_evidence_dump, on by default) shows WHY a
# skill did not install, not only that the resulting set was short.
installed() { # <args...>
    local target
    target="$(mktemp -d "$work/t.XXXXXX")"
    "$BASH" "$installer" "$@" --target "$target" --yes >"$target.log" 2>&1 || true
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
# The bare "one past the last skill" (what show_shop_menu prints as "all N
# skills") keeps meaning all, whole-string only: every number up to and
# including the skill count is also a position, and reading one as all inside
# a list would install one thing when a list was asked for. Computed, not
# hardcoded: this sentinel used to be the literal "6", from an era with six
# total menu entries, and silently stopped matching what the menu printed as
# skills were added to SKILL_NAMES -- typing the number the menu actually
# showed for "all" died "Unknown skill: <n>", while the disconnected literal
# "6" quietly still worked, matching nothing on screen.
skill_count="$("$BASH" -c 'source "'"$repo_root"'/installer/src/05-config.sh"; printf "%s" "${#SKILL_NAMES[@]}"')"
all_choice=$((skill_count + 1))
t_assert_eq 'the bare menu answer one past the last skill still means all' \
    "$(installed --skill "$all_choice")" "$every"
# The old literal "6" is now an ordinary position (whatever skill sits there),
# never a secret synonym for all -- the fix is that the sentinel moved to
# track the real count, not that "6" grew a second meaning alongside it.
t_assert_eq 'the number 6 alone now means skill number 6, not all' \
    "$(installed --skill 6)" "todo "

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

# The registry's parallel arrays must stay the same length. git-worktrees was
# added to SKILL_NAMES with no SKILL_DESCRIPTIONS entry, and moving the picker's
# cursor onto the last skill died with "IUI_SKILL_DESCS[$index]: unbound
# variable" under set -u. A registry that can go out of step silently is the
# defect; the crash was only its first symptom.
registry_lengths="$(
    # shellcheck disable=SC1090
    source "$repo_root/installer/src/05-config.sh"
    printf '%s %s %s' \
        "${#SKILL_NAMES[@]}" "${#SKILL_DESCRIPTIONS[@]}" "${#SKILL_DETAILS[@]}"
)"
skill_count="${registry_lengths%% *}"
t_assert_eq 'every skill has a one-line summary' \
    "$registry_lengths" "$skill_count $skill_count $skill_count"

t_end
