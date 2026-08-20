#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: DEV
# test-discovery-unit-target.sh — a discovery work unit may record File 'N/A'.
#
# SKILL.md 2.2 says to add a bounded discovery work unit exactly when the file or
# symbol is not yet knowable, and forbids TBD in a step. Both the writer and the
# validator nonetheless demanded a concrete file for every non-verification type,
# so the author had to invent one to satisfy the tool. Discovery may now use
# either: naming a file stays legal, because discovery often knows the file and
# not the symbol.
#
# The two guards mirror each other, and this pins both. Fixing one half only
# would leave a plan the writer accepts and the validator rejects.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/discovery-unit-target.XXXXXX")"
trap 'rm -rf "$work"' EXIT
plan="$work/plan"
cp -R "$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan" "$plan"

add_unit() { # <id> <type> <file> <step>
    "$scripts_dir/add-work-unit.sh" "$plan" --id "$1" --type "$2" --file "$3" \
        --scope backend --subscope N/A --change 'A probe row.' --depends-on '—' \
        --goal 01-plan-dir-synonym --step "$4" 2>&1
}

# ---- discovery may use N/A --------------------------------------------------
rc=0
message="$(add_unit W09 discovery N/A 05-step-locate-renderer)" || rc=$?
t_assert_eq 'a discovery unit with File N/A is accepted' "$rc" 0
grep -Fq '| W09 | discovery |' "$plan/work-unit-inventory.md" \
    || t_fail "the discovery row was not written: $message"

# ---- and the validator agrees, which is the half that could drift -----------
validation="$("$scripts_dir/validate-plan.sh" "$plan" 2>&1 || true)"
case "$validation" in
    *'W09'*'must name one file'*)
        t_fail 'the validator rejected the discovery row the writer accepted' ;;
esac

# ---- discovery may still name a file ---------------------------------------
# It often knows the file and not the symbol; N/A is permission, not a rule.
rc=0
add_unit W10 discovery 'planning/scripts/plan-env.sh' 06-step-locate-symbol >/dev/null 2>&1 || rc=$?
t_assert_eq 'a discovery unit naming a file is still accepted' "$rc" 0

# ---- every other type still must name one file -----------------------------
rc=0
message="$(add_unit W11 source N/A 07-step-source-probe)" || rc=$?
[ "$rc" -ne 0 ] || t_fail 'a source unit was allowed to use File N/A'
case "$message" in
    *'verification and discovery'*) ;;
    *) t_fail "the refusal did not name which types may use N/A: $message" ;;
esac

# ---- verification still must use N/A ---------------------------------------
rc=0
message="$(add_unit W12 verification 'planning/scripts/plan-env.sh' 08-step-verify-probe)" || rc=$?
[ "$rc" -ne 0 ] || t_fail 'a verification unit was allowed to name a file'
case "$message" in
    *'must use file N/A'*) ;;
    *) t_fail "the verification refusal changed shape: $message" ;;
esac

t_end
