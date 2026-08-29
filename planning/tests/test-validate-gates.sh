#!/usr/bin/env bash
# MODE: DEV
# test-validate-gates — the three gates are reported separately and each says
# something true about the run that produced it.
#
# Usage: test-validate-gates.sh
#
# A single pass/fail cannot describe a plan that is structurally sound, not yet
# adversarially approved, and nowhere near executed. Running --complete over a
# fully planned but unexecuted plan reported 218 errors, all of them execution
# state, and a reader looking for planning readiness took that as the verdict
# (T55). Two inferences are pinned here as much as the reporting: the structural
# gate is not claimed from an error count that mixes in execution state, and a
# plan with no review file is not reported as approved.
#
# This file is shipped, so it holds to the shipped-runtime dependency rule in
# CODE-STYLE.md §1: bash, POSIX coreutils, awk, sed, grep, rjq only. No python3.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scripts="$root/planning/scripts"

note_fail() { printf 'validate-gates: %s\n' "$1" >&2; t_record "$1"; }
# validate-plan exits non-zero on the failing path, which is the path this test
# cares about most, so the pipeline's status is deliberately discarded.
gates() { "$scripts/validate-plan.sh" "$@" 2>&1 | awk '/^Gates:/{print; exit}' || true; }
assert_has() { case "$2" in *"$1"*) ;; *) note_fail "$3: expected '$1' in: $2" ;; esac; }

work="$(mktemp -d "${TMPDIR:-/tmp}/validate-gates.XXXXXX")"
trap 'rm -rf "$work"' EXIT
export PLANS_ROOT="$work"
plan="$work/p"

"$scripts/create-plan.sh" p "Gates fixture" >/dev/null
"$scripts/add-goal.sh" "$plan" 01-a "A" "one demonstrable outcome" >/dev/null
"$scripts/add-work-unit.sh" "$plan" --id W01 --type source --file src/a.rs \
    --scope "a()" --subscope N/A --change "The one target." --depends-on -- \
    --goal 01-a --step 01-step-a >/dev/null

# 1. The line exists at all, on the failing path. A fresh plan fails structural
#    checks, and that is the run where a reader most needs to know which gate.
line="$(gates "$plan")"
[ -n "$line" ] || note_fail 'no Gates line on the failing path'
assert_has 'structurally valid=' "$line" 'structural gate named'
assert_has 'adversarially approved=' "$line" 'approval gate named'
assert_has 'implementation complete=' "$line" 'completion gate named'

# 2. No review file is not approval. review_approved is unset until a review is
#    read, and defaulting that to true reported a plan with no review as approved.
assert_has 'adversarially approved=no review file' "$line" 'missing review is not approval'

# 3. Without --complete the completion gate is not claimed either way.
assert_has 'implementation complete=not checked' "$line" 'completion not claimed by default'

# 4. Under --complete the structural verdict is refused rather than inferred:
#    the error count there mixes structural defects with execution state and the
#    checks interleave, so it cannot be separated after the fact.
complete_line="$(gates "$plan" --complete)"
assert_has 'structurally valid=not separable' "$complete_line" \
    'structural gate is not inferred from a --complete error count'
case "$complete_line" in
    *'implementation complete=not checked'*)
        note_fail '--complete still reported the completion gate as unchecked' ;;
esac

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'validate-gates: PASS\n'
