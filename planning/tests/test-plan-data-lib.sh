#!/usr/bin/env bash
# MODE: DEV
# test-plan-data-lib — contract tests for the plan-data readers in
# lib/core (compiled into plan-core-lib.sh): completion, cycle counting,
# unit counting. One mutation per contract must flip its own assertion.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
# shellcheck source=planning/scripts/plan-core-lib.sh
source "$LIB/plan-table-lib.sh"
source "$LIB/plan-core-lib.sh"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# ---- completion ----------------------------------------------------------
printf '| Goal | Progress |\n|---|---|\n| 01-a | 100%% |\n| 02-b | 100%% |\n' > "$tmp/full.md"
printf '| Goal | Progress |\n|---|---|\n| 01-a | 100%% |\n| 02-b | 40%% |\n' > "$tmp/partial.md"
: > "$tmp/empty.md"
t_assert_eq "complete inventory completes" \
  "$(plan_progress_is_complete "$tmp/full.md" && echo yes || echo no)" "yes"
t_assert_eq "partial inventory incomplete" \
  "$(plan_progress_is_complete "$tmp/partial.md" && echo yes || echo no)" "no"
t_assert_eq "empty roster incomplete" \
  "$(plan_progress_is_complete "$tmp/empty.md" && echo yes || echo no)" "no"
t_assert_eq "missing file incomplete without abort" \
  "$(plan_progress_is_complete "$tmp/nope.md" && echo yes || echo no)" "no"

# ---- cycle count ---------------------------------------------------------
printf '## Cycle 1\nx\n## Cycle 2\ny\n## Cycle 3\nz\n' > "$tmp/three.md"
printf '' > "$tmp/nocycles.md"
printf 'intro\r\n## Cycle 1\r\nbody\r\n' > "$tmp/crlf.md"
t_assert_eq "three cycles counted" "$(plan_cycle_count "$tmp/three.md")" "3"
t_assert_eq "zero cycles yields zero" "$(plan_cycle_count "$tmp/nocycles.md")" "0"
t_assert_eq "crlf normalised before counting" "$(plan_cycle_count "$tmp/crlf.md")" "1"
t_assert_eq "missing document yields zero" "$(plan_cycle_count "$tmp/nope.md")" "0"

# ---- unit count ----------------------------------------------------------
cat > "$tmp/inv.md" <<'INV'
## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| outcome one | W01,W02 | notes |

## Work units

| ID | Type | File |
|---|---|---|
| W01 | source | a.sh |
| W02 | test | b.sh |

INV
t_assert_eq "unit rows counted, coverage and separators ignored" \
  "$(plan_count_units "$tmp/inv.md")" "2"
: > "$tmp/nounits.md"
t_assert_eq "zero unit rows yields zero" "$(plan_count_units "$tmp/nounits.md")" "0"

t_end
