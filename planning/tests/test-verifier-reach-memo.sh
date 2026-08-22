#!/usr/bin/env bash
# MODE: DEV
# B18 regression — the verifier-reachability memo must not poison later checks.
#
# Two faults composed into one false FAIL. dep_reaches wrote every visited pair
# into its map at entry and answered any later hit with failure, so a pair that
# had been probed successfully left behind a key that read back as "known
# failure"; and the caller's reset (`dep_seen=()`) cleared a variable the map
# never used — plan maps live in plan_map__<map>__<key> variables, so nothing
# was reset at all. An early pair check therefore fixed both directions of a
# pair into the memo, and the later direct probe of the same pair failed it
# despite valid edges:
#
#   scan W02 first (its step mentions W04): forward probe W02->W04 fails,
#   writing W02/W04; the elif reverse probe W04->W02 succeeds over the direct
#   edge, leaving W04/W02.
#   scan W04 next (its step mentions W02): forward W04->W02 hits the memo -> 1;
#   elif W02->W04 hits the memo -> 1; "no dependency path" for a pair joined by
#   a direct dependency edge.
#
# The positive control keeps the real gap detectable: a verifier whose grader
# target has no edge to it at all must still be reported.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin
fail() { t_fail "$*"; }
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-reach-memo-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

build_goal_fixture() { # <plan-dir>
    local plan="$1"
    "$script_dir/create-plan.sh" "$plan" reach-memo >/dev/null
    "$script_dir/add-goal.sh" "$plan" 01-g 'G' 'an outcome' >/dev/null
    "$script_dir/add-work-unit.sh" "$plan" W01 source a.php 'A::x' N/A 'change A' '—' 01-g 01-step-a >/dev/null
    "$script_dir/add-work-unit.sh" "$plan" W02 verification N/A 'v-two.sh' N/A 'Baseline capture before W04.' '—' 01-g 02-step-v2 >/dev/null
    "$script_dir/add-work-unit.sh" "$plan" W04 verification N/A 'v-four.sh' N/A 'Verify W02.' 'W02' 01-g 04-step-v4 >/dev/null
    "$script_dir/update-plan-content.sh" --testing-requirement "$plan" 01-g yes 'verification units present' >/dev/null
    printf '# V\n\n## Automated tests\n\nx\n' > "$plan/01-g/steps/01-step-a-testing.md"
    printf '# V\n\n## Automated tests\n\nx\n' > "$plan/01-g/steps/02-step-v2-testing.md"
    printf '# V\n\n## Automated tests\n\nx\n' > "$plan/01-g/steps/04-step-v4-testing.md"
    "$script_dir/add-coverage.sh" "$plan" 'A changed.' W01 'covered' >/dev/null
    "$script_dir/add-coverage.sh" "$plan" 'Baseline captured.' W02 'covered' >/dev/null
    "$script_dir/add-coverage.sh" "$plan" 'Verified.' W04 'covered' >/dev/null
    "$script_dir/create-adversarial-review.sh" "$plan" >/dev/null
}

# ---- B18: mutual mentions plus a direct reverse edge must pass both ways ----
plan="$temporary_root/mutual"
build_goal_fixture "$plan"
if "$script_dir/validate-plan.sh" "$plan" > "$temporary_root/mutual.log" 2>&1; then :; fi
t_assert_eq \
    "mutual verifiers joined by a direct edge produce no reachability finding" \
    "$(grep -c 'no dependency path' "$temporary_root/mutual.log" || true)" 0

# ---- positive control: a real grader gap must still be reported ----
control="$temporary_root/control"
"$script_dir/create-plan.sh" "$control" reach-gap >/dev/null
"$script_dir/add-goal.sh" "$control" 01-g 'G' 'an outcome' >/dev/null
"$script_dir/add-work-unit.sh" "$control" W01 source b.php 'B::x' N/A 'change B' '—' 01-g 01-step-b >/dev/null
"$script_dir/add-work-unit.sh" "$control" W02 verification N/A 'v-two.sh' N/A 'Verify W01.' '—' 01-g 02-step-v2 >/dev/null
"$script_dir/update-plan-content.sh" --testing-requirement "$control" 01-g yes 'verification unit present' >/dev/null
printf '# V\n\n## Automated tests\n\nx\n' > "$control/01-g/steps/01-step-b-testing.md"
printf '# V\n\n## Automated tests\n\nx\n' > "$control/01-g/steps/02-step-v2-testing.md"
"$script_dir/add-coverage.sh" "$control" 'B changed.' W01 'covered' >/dev/null
"$script_dir/add-coverage.sh" "$control" 'Verified.' W02 'covered' >/dev/null
"$script_dir/create-adversarial-review.sh" "$control" >/dev/null
if "$script_dir/validate-plan.sh" "$control" > "$temporary_root/control.log" 2>&1; then :; fi
t_assert_eq \
    "a verifier with no dependency path is still reported" \
    "$(grep -c 'W02 is a verification unit that grades W01 but has no dependency path' "$temporary_root/control.log" || true)" 1

t_end
