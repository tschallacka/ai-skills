#!/usr/bin/env bash
# MODE: DEV
# test-roster-cross-reference.sh — the §9.x roster check reads the id that heads
# each blurb, not every id mentioned in its description.
#
# The blurb pass stripped every backtick and harvested all WNN tokens on the
# line, so a description that legitimately cross-references another unit ("as
# `W05` does") was read as the goal claiming to own it, and validation failed
# with a roster/inventory mismatch that did not exist. Reported by a worker
# against the installed skill; the same code was in the repo.
#
# Both directions matter: the false positive has to go, and a blurb that really
# does head a unit the inventory assigns elsewhere must still fail.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/roster-cross-reference.XXXXXX")"
trap 'rm -rf "$work"' EXIT
fixture="$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan"

fresh_plan() { # <name> -> a copy of the fixture
    local plan="$work/$1"
    rm -rf "$plan"
    cp -R "$fixture" "$plan"
    printf '%s\n' "$plan"
}

# The goal owns W01-W04; the inventory assigns W05 to the second goal.
owned_head='`W01` — No change; the hoister already exists and is the seam the other units use.'

# ---- the premise: the fixture validates as it stands ------------------------
plan="$(fresh_plan baseline)"
"$scripts_dir/validate-plan.sh" "$plan" >/dev/null 2>&1 \
    || t_fail 'the fixture plan does not validate, so neither case below means anything'

# ---- a cross-reference in a description is not an ownership claim -----------
plan="$(fresh_plan cross-reference)"
goal_file="$plan/01-plan-dir-synonym/goal.md"
replacement="$owned_head It builds on \`W05\`, which the other goal owns."
awk -v old="$owned_head" -v new="$replacement" \
    '$0 == old { print new; next } { print }' "$goal_file" > "$goal_file.tmp"
mv "$goal_file.tmp" "$goal_file"
grep -Fq 'W05' "$goal_file" || t_fail 'the cross-reference was not written, so nothing is being tested'
rc=0
message="$("$scripts_dir/validate-plan.sh" "$plan" 2>&1)" || rc=$?
if [ "$rc" -ne 0 ]; then
    printf 'a description cross-referencing another goal unit failed validation:\n%s\n' "$message" >&2
    t_fail 'a legitimate cross-reference was read as a roster claim'
fi

# ---- a blurb that really heads a foreign unit still fails -------------------
plan="$(fresh_plan mis-claimed)"
goal_file="$plan/01-plan-dir-synonym/goal.md"
awk '
    { print }
    /^`W01` —/ && !done {
        print ""
        print "§ 9.5"
        print "`W05` — claimed here although the inventory assigns it elsewhere."
        done = 1
    }
' "$goal_file" > "$goal_file.tmp"
mv "$goal_file.tmp" "$goal_file"
rc=0
message="$("$scripts_dir/validate-plan.sh" "$plan" 2>&1)" || rc=$?
[ "$rc" -ne 0 ] || t_fail 'a blurb heading a unit the goal does not own was accepted'
case "$message" in
    *W05*) ;;
    *) t_fail "the refusal did not name the mis-claimed unit: $message" ;;
esac

# ---- an empty roster must not abort the run before any verdict (B36) --------
# A goal whose §9.x section holds no bare `WNN` blurb leaves both awk passes
# empty; grep then matches nothing, and its exit 1 under pipefail used to abort
# validate-plan mid-pass with no summary at all. An empty match is a real
# answer: the run continues and reports the roster omission like any other.
plan="$(fresh_plan empty-roster)"
goal_file="$plan/01-plan-dir-synonym/goal.md"
grep -v '^`W[0-9][0-9]` —' "$goal_file" > "$goal_file.tmp"
mv "$goal_file.tmp" "$goal_file"
if grep -qE '^`W[0-9][0-9]`' "$goal_file"; then
    t_fail 'the empty-roster fixture still carries a bare work-unit id'
fi
rc=0
message="$("$scripts_dir/validate-plan.sh" "$plan" 2>&1)" || rc=$?
[ "$rc" -ne 0 ] || t_fail 'an empty roster validated clean'
case "$message" in
    *'roster omits'*) ;;
    *) t_fail "the roster check never ran: $message" ;;
esac
case "$message" in
    *'Plan validation failed'*) ;;
    *) t_fail "the run aborted before a verdict printed: $message" ;;
esac

t_end
