#!/usr/bin/env bash
# test-obsolete-plan.sh — a plan from an older skill version is refused, kept.
#
# The skill does no backwards compatibility, so a plan directory built by an
# older version is marked obsolete and rebuilt as a new plan rather than
# migrated. validate-plan.sh must refuse an obsolete plan by name — exit 65,
# naming the replacement — instead of reporting a wall of findings about a plan
# nobody should be using, and it must never remove anything.
set -euo pipefail
export LC_ALL=C

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture_plan="$repo_root/benchmark/planning/tests/fixtures/review-lifecycle-plan"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-obsolete-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

failures=0
pass() { printf 'PASS: %s\n' "$1"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; failures=$((failures + 1)); }

# run_validate <plan> <log> [flag…] — prints the exit code, never aborts.
run_validate() {
    local plan="$1" log="$2" code=0
    shift 2
    "$script_dir/validate-plan.sh" ${1+"$@"} "$plan" > "$log" 2>&1 || code="$?"
    printf '%s' "$code"
}

assert_grep() {
    local log="$1" needle="$2" label="$3"
    if grep -Fq "$needle" "$log"; then
        pass "$label"
    else
        fail "$label (log has no '$needle')"
        sed 's/^/    /' "$log" >&2
    fi
}

plan="$temporary_root/plan"
t_copy_tree "$fixture_plan" "$plan"

# Control: the committed fixture is a plan that validates clean today, so a 65
# afterwards can only come from the marker.
code="$(run_validate "$plan" "$temporary_root/before.log")"
if [ "$code" = 0 ]; then
    pass 'the fixture plan validates before it is marked obsolete'
else
    fail "the fixture plan did not validate before being marked obsolete (exit $code)"
    sed 's/^/    /' "$temporary_root/before.log" >&2
fi

# The marker is a file in the plan directory, written by hand at the moment the
# rebuild starts. It names the plan that supersedes this one.
{
    printf 'obsoleted-at: 2026-08-19\n'
    printf 'obsoleted-because: built by an older planning-skill version\n'
    printf 'replaced-by: 2026-08-19-review-lifecycle-rebuild\n'
} > "$plan/OBSOLETE"

for mode in plain complete; do
    log="$temporary_root/obsolete-$mode.log"
    if [ "$mode" = complete ]; then
        code="$(run_validate "$plan" "$log" --complete)"
    else
        code="$(run_validate "$plan" "$log")"
    fi
    if [ "$code" = 65 ]; then
        pass "an obsolete plan is refused with exit 65 ($mode)"
    else
        fail "an obsolete plan exited $code instead of 65 ($mode)"
        sed 's/^/    /' "$log" >&2
    fi
    assert_grep "$log" 'is marked obsolete' "the refusal says the plan is obsolete ($mode)"
    assert_grep "$log" '2026-08-19-review-lifecycle-rebuild' "the refusal names the replacement ($mode)"
    assert_grep "$log" 'nothing here was deleted' "the refusal states that nothing was deleted ($mode)"
    if grep -q '^FAIL:' "$log"; then
        fail "the refusal reported plan findings as well ($mode)"
        sed 's/^/    /' "$log" >&2
    else
        pass "the refusal reports no findings from a plan nobody should use ($mode)"
    fi
done

# Nothing is deleted, ever — including by the gate that refuses the plan.
for kept in plan-description.md work-unit-inventory.md adversarial-review.md OBSOLETE; do
    if [ -f "$plan/$kept" ]; then
        pass "$kept survives the refusal"
    else
        fail "$kept was removed from an obsolete plan"
    fi
done

# A marker that names no replacement is still a refusal, and says what is missing.
printf 'obsoleted-at: 2026-08-19\n' > "$plan/OBSOLETE"
log="$temporary_root/obsolete-unnamed.log"
code="$(run_validate "$plan" "$log")"
if [ "$code" = 65 ]; then
    pass 'a marker with no replacement is refused with exit 65'
else
    fail "a marker with no replacement exited $code instead of 65"
    sed 's/^/    /' "$log" >&2
fi
assert_grep "$log" 'names no replacement' 'the refusal asks for the replaced-by line'

# Removing the marker restores an ordinary validation: the refusal is the
# marker's doing, not a permanent property of the directory.
rm -f "$plan/OBSOLETE"
code="$(run_validate "$plan" "$temporary_root/after.log")"
if [ "$code" = 0 ]; then
    pass 'the plan validates again once the marker is gone'
else
    fail "the plan did not validate after the marker was removed (exit $code)"
    sed 's/^/    /' "$temporary_root/after.log" >&2
fi

# The instruction that produces the marker must be in SKILL.md, not only here.
skill="$repo_root/planning/SKILL.md"
for phrase in 'OBSOLETE' 'replaced-by:' 'Nothing is deleted'; do
    if grep -Fq "$phrase" "$skill"; then
        pass "SKILL.md documents '$phrase'"
    else
        fail "SKILL.md does not document '$phrase'"
    fi
done

if [ "$failures" -ne 0 ]; then
    printf 'test-obsolete-plan: %d failure(s).\n' "$failures" >&2
    exit 1
fi
printf 'test-obsolete-plan.sh passed.\n'
