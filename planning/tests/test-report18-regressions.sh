#!/usr/bin/env bash
# Regression tests for planning-skill report 18:
#   §2: add-work-unit.sh emits the next free § 9.N label per unit — four
#       helper-built units produce 9.1..9.4 in order, never duplicate 9.2
#   §3: update-work-unit.sh — an empty positional scope leaves scope
#       unchanged instead of shifting the file into it; --file flag exists
#   §4: create-step-testing.sh labels EVERY paragraph of a companion, whether
#       paragraphs are separated by real newlines or literal "\n" escapes
#   §1: the next-free-number paragraph write is created silently and anything
#       beyond it is refused, identically on step documents and companions

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

# A finding no longer stops the run: this file used a byte-identical copy of
# the library's reporter that exited on the first one, so a broken subject
# reported one failure and hid the rest.
fail() { t_fail "$*"; }
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-report18-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT


# ---- §2: four helper-built units get 9.1, 9.2, 9.3, 9.4 ----
plan_units="$temporary_root/units"
"$script_dir/create-plan.sh" "$plan_units" 'Unit labels' >/dev/null
"$script_dir/add-goal.sh" "$plan_units" 00-g 'G' 'an outcome' >/dev/null
add_unit() {
    "$script_dir/add-work-unit.sh" "$plan_units" "$@" >/dev/null 2>&1
}
add_unit W01 source app/code/X/A.php 'A::run()' N/A 'a' — 00-g 01-step-a
add_unit W02 test app/code/X/ATest.php 'ATest' N/A 'b' W01 00-g 02-step-b
add_unit W03 verification N/A 'browser check' N/A 'verify' W01,W02 00-g 03-step-verify
add_unit W04 config app/code/X/etc/di.xml 'type X' N/A 'wire' W01 00-g 04-step-wire
labels="$(grep -E '^§ 9\.' "$plan_units/00-g/goal.md")"
[ "$(printf '%s\n' "$labels" | wc -l)" -eq 4 ] || fail "expected 4 § 9.N labels, got: $(printf '%s\n' "$labels")"
[ "$(printf '%s\n' "$labels" | sort | uniq -d | wc -l)" -eq 0 ] || fail 'duplicate § 9.N labels'
[ "$(printf '%s\n' "$labels" | tr '\n' ' ')" = '§ 9.1 § 9.2 § 9.3 § 9.4 ' ] \
    || fail "labels not ascending 9.1..9.4: $(printf '%s\n' "$labels" | tr '\n' ' ')"
label_report="$("$script_dir/validate-plan.sh" "$plan_units" 2>&1 || true)"
case "$label_report" in
    *'duplicate paragraph label'*) fail 'validator still reports duplicate § 9 labels' ;;
esac

# ---- §3: empty positional scope + --file flag ----
"$script_dir/update-work-unit.sh" "$plan_units" W01 '' 'app/code/X/Moved.php' >/dev/null
row="$(grep '^| W01 |' "$plan_units/work-unit-inventory.md")"
case "$row" in
    *'app/code/X/Moved.php'*) ;;
    *) fail 'empty-scope positional did not update File' ;;
esac
case "$row" in
    *'`A::run()`'*) ;;
    *) fail 'empty-scope positional shifted the file into Primary scope' ;;
esac
"$script_dir/update-work-unit.sh" "$plan_units" W02 --file 'app/code/X/MovedTest.php' >/dev/null
row="$(grep '^| W02 |' "$plan_units/work-unit-inventory.md")"
case "$row" in
    *'app/code/X/MovedTest.php'*) ;;
    *) fail '--file flag did not update File' ;;
esac
# The classic positional pair still works.
"$script_dir/update-work-unit.sh" "$plan_units" W02 'newscope' 'app/code/X/Third.php' >/dev/null
row="$(grep '^| W02 |' "$plan_units/work-unit-inventory.md")"
case "$row" in
    *'newscope'*) ;;
    *) fail 'first positional no longer updates scope' ;;
esac
case "$row" in
    *'app/code/X/Third.php'*) ;;
    *) fail 'second positional no longer updates File' ;;
esac

# ---- §4: every companion paragraph is labeled ----
plan_comp="$temporary_root/companion"
"$script_dir/create-plan.sh" "$plan_comp" 'Companion' >/dev/null
mkdir -p "$plan_comp/00-g/steps"
printf '# Step: s\n\n## Instructions\n\n§ 5.1\nx\n' > "$plan_comp/00-g/steps/01-step-a.md"
"$script_dir/create-step-testing.sh" "$plan_comp/00-g" 01-step-a \
    'First para.

Second para.

Third para.

Fourth para.

Fifth para.' >/dev/null
companion="$plan_comp/00-g/steps/01-step-a-testing.md"
[ "$(grep -c '^§ ' "$companion")" -eq 5 ] || fail "real-newline companion has $(grep -c '^§ ' "$companion") labels, expected 5"
for para in 'First para.' 'Second para.' 'Third para.' 'Fourth para.' 'Fifth para.'; do
    grep -Fq "$para" "$companion" || fail "companion lost paragraph: $para"
done
grep -Fqx '§ 2.5' "$companion" || fail 'last companion paragraph is unlabeled'
# Literal "\n" escapes still label every paragraph.
"$script_dir/create-step-testing.sh" "$plan_comp/00-g" 01-step-a 'a\ntwo\n\nthree' --overwrite >/dev/null
[ "$(grep -c '^§ ' "$companion")" -eq 3 ] || fail "literal-\\n companion has $(grep -c '^§ ' "$companion") labels, expected 3"

# ---- §1: next-free created, beyond refused, identical on both document types ----
plan_next="$temporary_root/next-free"
"$script_dir/create-plan.sh" "$plan_next" 'Next free' >/dev/null
"$script_dir/add-goal.sh" "$plan_next" 00-g 'G' 'an outcome' >/dev/null
mkdir -p "$plan_next/00-g/steps"
printf '# Step: s\n\n## Instructions\n\n§ 5.1\na\n\n§ 5.2\nb\n' > "$plan_next/00-g/steps/01-step-s.md"
"$script_dir/update-plan-content.sh" -sp "$plan_next" 00-g/01-step-s 5.3 'c' >/dev/null 2>&1 \
    || fail 'next-free paragraph on a step was refused'
if "$script_dir/update-plan-content.sh" -sp "$plan_next" 00-g/01-step-s 5.7 'x' >/dev/null 2>&1; then
    fail 'far-out paragraph on a step was silently created'
fi
"$script_dir/create-step-testing.sh" "$plan_next/00-g" 01-step-s 'one' >/dev/null
"$script_dir/update-plan-content.sh" -sp "$plan_next" 00-g/01-step-s-testing 2.2 'two' >/dev/null 2>&1 \
    || fail 'next-free paragraph on a companion was refused'
if "$script_dir/update-plan-content.sh" -sp "$plan_next" 00-g/01-step-s-testing 2.9 'x' >/dev/null 2>&1; then
    fail 'far-out paragraph on a companion was silently created'
fi

t_end