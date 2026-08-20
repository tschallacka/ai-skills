#!/usr/bin/env bash
# test-lib-progress.sh — the progress functions, each sourced on its own.
#
# The second of two layers, and deliberately overlapping with the first. The
# integration tests exercise these through the compiled library and the entry
# points that call it; this one sources a single function file, stubs whatever it
# calls, and pins the function's own behaviour. When both fail, the unit test
# says which function is wrong and the integration test says what it broke.
#
# Sourcing one file at a time is what the T4 split bought. Each case runs in a
# subshell so a stub cannot leak into the next.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
lib="$repo_root/planning/scripts/lib/progress"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

# Run one expression with only the named function files sourced. Stubs go in the
# body, before the call, so the unit under test never reaches a real sibling.
unit() { # <file>... -- <expression>
    local files=() f
    while [ "$#" -gt 0 ] && [ "$1" != '--' ]; do files+=("$1"); shift; done
    shift
    local prelude=''
    for f in "${files[@]}"; do prelude="$prelude source '$lib/$f';"; done
    "$BASH" -c "set -uo pipefail; $prelude $1" 2>&1
}

# ── plan_progress_percent: half-up rounding, and zero total ──────────────────
t_assert_eq 'percent 0 of 10' "$(unit plan_progress_percent.sh -- 'plan_progress_percent 0 10')" '0'
t_assert_eq 'percent 5 of 10' "$(unit plan_progress_percent.sh -- 'plan_progress_percent 5 10')" '50'
t_assert_eq 'percent 10 of 10' "$(unit plan_progress_percent.sh -- 'plan_progress_percent 10 10')" '100'
# 1 of 3 is 33.33; half-up gives 33. 2 of 3 is 66.67; half-up gives 67. The
# rounding is part of the on-disk contract, so both directions are pinned.
t_assert_eq 'percent rounds down at 1 of 3' "$(unit plan_progress_percent.sh -- 'plan_progress_percent 1 3')" '33'
t_assert_eq 'percent rounds up at 2 of 3' "$(unit plan_progress_percent.sh -- 'plan_progress_percent 2 3')" '67'
# A zero total must not divide by zero: the seed state of every new plan.
t_assert_eq 'percent of an empty plan' "$(unit plan_progress_percent.sh -- 'plan_progress_percent 0 0')" '0'

# ── plan_progress_bar: width, glyphs, and the percent it is handed ───────────
# The stub is the point: the bar's arithmetic is tested without the real
# percent function, so a bar failure cannot be a percent failure in disguise.
t_assert_eq 'bar at 50 percent' \
    "$(unit plan_progress_bar.sh -- 'plan_progress_percent() { printf 50; }; plan_progress_bar 1 2 20')" \
    '##########----------'
t_assert_eq 'bar at 0 percent is all empty' \
    "$(unit plan_progress_bar.sh -- 'plan_progress_percent() { printf 0; }; plan_progress_bar 0 5 20')" \
    '--------------------'
t_assert_eq 'bar at 100 percent is all filled' \
    "$(unit plan_progress_bar.sh -- 'plan_progress_percent() { printf 100; }; plan_progress_bar 5 5 20')" \
    '####################'
t_assert_eq 'bar honours a narrower width' \
    "$(unit plan_progress_bar.sh -- 'plan_progress_percent() { printf 50; }; plan_progress_bar 1 2 4')" \
    '##--'
t_assert_eq 'bar defaults to 20 columns' \
    "$(unit plan_progress_bar.sh -- 'plan_progress_percent() { printf 100; }; plan_progress_bar 1 1 | tr -d "\n" | wc -c | tr -d " "')" \
    '20'

# ── plan_progress_icon: the three glyphs are an on-disk contract ─────────────
t_assert_eq 'icon before anything is done' "$(unit plan_progress_icon.sh -- 'plan_progress_icon 0 0')" '💤'
t_assert_eq 'icon in progress' "$(unit plan_progress_icon.sh -- 'plan_progress_icon 1 50')" '⏳'
t_assert_eq 'icon complete' "$(unit plan_progress_icon.sh -- 'plan_progress_icon 5 100')" '✅'
# 100 percent with nothing completed cannot happen from a real count, but the
# function must not disagree with itself if it does.
t_assert_eq 'icon prefers complete over untouched' "$(unit plan_progress_icon.sh -- 'plan_progress_icon 0 100')" '✅'

# ── plan_status_label: an unknown word is a refusal, not a default ───────────
t_assert_eq 'label incomplete' "$(unit plan_status_label.sh -- 'plan_status_label incomplete')" '💤 incomplete'
t_assert_eq 'label in-progress' "$(unit plan_status_label.sh -- 'plan_status_label in-progress')" '⏳ in progress'
t_assert_eq 'label in_progress underscore form' "$(unit plan_status_label.sh -- 'plan_status_label in_progress')" '⏳ in progress'
t_assert_eq 'label completed' "$(unit plan_status_label.sh -- 'plan_status_label completed')" '✅ completed'
rc=0
unit plan_status_label.sh -- 'plan_status_label nonsense' >/dev/null 2>&1 || rc=$?
[ "$rc" -ne 0 ] || t_fail 'plan_status_label accepted a status word it does not know'

# ── plan_step_objective: the paragraph, or the fallback ──────────────────────
work="$(mktemp -d "${TMPDIR:-/tmp}/lib-progress.XXXXXX")"
trap 'rm -rf "$work"' EXIT
printf '# Step\n\n## Objective\n\n%s 5.1\nThe objective sentence.\n\n## Instructions\n' '§' > "$work/step.md"
t_assert_eq 'objective reads the paragraph under the heading' \
    "$(unit plan_step_objective.sh -- "plan_step_objective '$work/step.md' 'unused fallback'")" \
    'The objective sentence.'
printf '# Step\n\n## Instructions\n\nno objective here\n' > "$work/no-objective.md"
t_assert_eq 'objective falls back when the heading is absent' \
    "$(unit plan_step_objective.sh -- "plan_step_objective '$work/no-objective.md' 'the fallback'")" \
    'the fallback'

# ── plan_emit_step_testing_reminder: only for a step or unit id ──────────────
# Its early return is the whole contract for a plan-level id, and it is the one
# branch reachable without a plan on disk.
t_assert_eq 'no reminder for a plan-level id' \
    "$(unit plan_emit_step_testing_reminder.sh -- 'plan_document_path() { printf unused; }; plan_emit_step_testing_reminder /nonexistent plan; printf done')" \
    'done'
t_assert_eq 'no reminder for an inventory id' \
    "$(unit plan_emit_step_testing_reminder.sh -- 'plan_document_path() { printf unused; }; plan_emit_step_testing_reminder /nonexistent inventory; printf done')" \
    'done'

t_end
