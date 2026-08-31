#!/usr/bin/env bash
# MODE: DEV
# Fixture corpus contracts for the plan overview's degenerate inputs.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/fixtures/overview" && pwd)"

for fixture in navigation size anomalies evidence-gaps complete empty-approved fresh malformed-state; do
    [ -f "$root/$fixture/FIXTURE-VERSION" ] || t_fail "$fixture has no provenance record"
done

t_assert_contains 'navigation snapshot' 'source=plan-overview-rebuild' "$(cat "$root/navigation/FIXTURE-VERSION")"
t_assert_contains 'navigation snapshot' 'units=82' "$(cat "$root/navigation/FIXTURE-VERSION")"
t_assert_contains 'size snapshot' 'state_bytes=341088' "$(cat "$root/size/FIXTURE-VERSION")"
t_assert_contains 'anomalies cases' 'orphan' "$(cat "$root/anomalies/FIXTURE-VERSION")"
t_assert_contains 'anomalies cases' 'testing-requirement-no' "$(cat "$root/anomalies/FIXTURE-VERSION")"
t_assert_contains 'evidence gaps' 'blank-work-unit' "$(cat "$root/evidence-gaps/FIXTURE-VERSION")"
t_assert_contains 'evidence gaps' 'gated-fix-key' "$(cat "$root/evidence-gaps/FIXTURE-VERSION")"
t_assert_contains 'evidence gaps' 'uncovered-outcome' "$(cat "$root/evidence-gaps/FIXTURE-VERSION")"
t_assert_contains 'evidence gaps' 'empty-review-cycle' "$(cat "$root/evidence-gaps/FIXTURE-VERSION")"
t_assert_contains 'complete shape' 'approved-all-passed' "$(cat "$root/complete/FIXTURE-VERSION")"
t_assert_contains 'empty approved shape' 'zero-steps' "$(cat "$root/empty-approved/FIXTURE-VERSION")"
t_assert_contains 'fresh zeroes' 'no-findings,no-completed-steps,no-review-cycles' "$(cat "$root/fresh/FIXTURE-VERSION")"
t_assert_contains 'malformed damage' 'parse=skip' "$(cat "$root/malformed-state/FIXTURE-VERSION")"
t_assert_contains 'malformed damage' 'truncated-state,transition-without-time' "$(cat "$root/malformed-state/FIXTURE-VERSION")"
[ "$(wc -c < "$root/size/FIXTURE-VERSION" | tr -d ' ')" -gt 0 ] || t_fail 'size provenance is empty'
t_end
