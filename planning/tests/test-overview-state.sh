#!/usr/bin/env bash
# MODE: DEV
# test-overview-state.sh — pins the reviewing-state JSON shape that both
# delivery modes render from (T43c). The fixture is synthesised through the
# sanctioned helpers into a temp dir, never read from gitignored .plans.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "planning/tests/lib-test.sh" 2>/dev/null || source "/home/tschallacka/git/ai-skills/planning/tests/lib-test.sh"
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root_tests="$(cd "$tests_dir/../.." && pwd)"
scripts="$repo_root_tests/planning/scripts"
state="$scripts/overview-state.sh"
work="$(mktemp -d "${TMPDIR:-/tmp}/overview-state.XXXXXX")"
trap 'rm -rf "$work"' EXIT

fail() { printf 'overview-state: %s\n' "$1" >&2; FAILED=1; }
FAILED=0

# ---- synthesise a complete fixture plan --------------------------------------
plan="$work/fixture"
export PLANS_ROOT="$work/plans-root"
"$BASH" "$scripts/create-plan.sh" "$plan" 'State extraction fixture' >/dev/null
"$BASH" "$scripts/add-goal.sh" "$plan" 01-build 'Build the widget' 'The widget exists and works.' >/dev/null
"$BASH" "$scripts/add-work-unit.sh" "$plan" --id W01 --type source --file src/widget.php \
    --scope 'Widget::render()' --subscope N/A --change 'Render the widget' \
    --depends-on '—' --goal 01-build --step 01-step-render >/dev/null
"$BASH" "$scripts/add-work-unit.sh" "$plan" --id W02 --type verification --file N/A \
    --scope 'verify-widget' --subscope N/A --change 'Verify the widget renders' \
    --depends-on 'W01' --goal 01-build --step 02-step-verify >/dev/null
"$BASH" "$scripts/update-plan-content.sh" --testing-requirement "$plan" 01-build yes 'verification unit present' >/dev/null
printf '# t\n\n## Automated tests\n\nx\n' > "$plan/01-build/steps/01-step-render-testing.md"
"$BASH" "$scripts/add-coverage.sh" "$plan" 'The widget renders' W01,W02 'covered by render plus verify' >/dev/null
"$BASH" "$scripts/create-adversarial-review.sh" "$plan" >/dev/null

out="$(OVERVIEW_NOW=2026-08-22T1200Z bash "$state" --plan-dir "$plan")"

# ---- every top-level key exists ----------------------------------------------
for key in identity goals steps edges testingMarks coverage findings cycles reviewTarget generatedBy; do
    jq -e --arg k "$key" 'has($k)' <<<"$out" >/dev/null \
        || fail "top-level key '$key' missing from state JSON"
done

# ---- identity carries title, ui flag, review status ---------------------------
jq -e '.identity.title == "State extraction fixture"' <<<"$out" >/dev/null \
    || fail "identity.title wrong"
jq -e '.identity.uiAffected | startswith("yes") or startswith("no")' <<<"$out" >/dev/null \
    || fail "identity.uiAffected missing"

# ---- goals carry id, outcome, testing requirement -----------------------------
jq -e '.goals | length == 1' <<<"$out" >/dev/null || fail "expected exactly one goal"
jq -e '.goals[0].id == "01-build"' <<<"$out" >/dev/null || fail "goal id wrong"
jq -e '(.goals[0].outcome | length) > 0' <<<"$out" >/dev/null \
    || fail "goal outcome missing"
jq -e '.goals[0].testingRequirement | contains("yes")' <<<"$out" >/dev/null \
    || fail "per-goal testing requirement missing (AR-15)"

# ---- steps carry instructions, criteria, status, unit, companion --------------
jq -e '.steps | length == 2' <<<"$out" >/dev/null || fail "expected two steps"
jq -e '[.steps[].unit] == ["W01","W02"]' <<<"$out" >/dev/null || fail "step units wrong order or value"
jq -e '.steps[0].companion == "01-step-render-testing.md"' <<<"$out" >/dev/null \
    || fail "companion for 01-step-render missing"
jq -e '.steps[1].companion == null' <<<"$out" >/dev/null || fail "unexpected companion on verify step"
jq -e '.steps[0].instructions | length > 0' <<<"$out" >/dev/null || fail "instructions missing"
jq -e '.steps[0].criteria | length > 0' <<<"$out" >/dev/null || fail "acceptance criteria missing"

# ---- dependency edges reference existing units --------------------------------
jq -e '.edges | length == 1' <<<"$out" >/dev/null || fail "expected one edge"
jq -e '.edges[0].from == "W02"' <<<"$out" >/dev/null || fail "edge from wrong"
jq -e '.edges[0].to == "W01"' <<<"$out" >/dev/null || fail "edge to wrong"

# ---- coverage rows present -----------------------------------------------------
jq -e '.coverage | length >= 1' <<<"$out" >/dev/null || fail "coverage rows missing"
jq -e '.coverage[0].units == "W01,W02"' <<<"$out" >/dev/null || fail "coverage units wrong"

# ---- findings: empty live table still yields the array -------------------------
jq -e '.findings | type == "array"' <<<"$out" >/dev/null || fail "findings array missing"

# ---- review baseline of two (T43e) --------------------------------------------
jq -e '.reviewTarget == 2' <<<"$out" >/dev/null || fail "review target must be 2"

# ---- escaping: angle brackets survive as data, never break the JSON -----------
"$BASH" "$scripts/add-adversarial-finding.sh" "$plan" AR-05 '<script>alert(1)</script>' 'escape it' resolved --work-unit W01 >/dev/null 2>&1 || true
out="$(OVERVIEW_NOW=2026-08-22T1200Z bash "$state" --plan-dir "$plan")"
jq -e '.findings | length >= 1' <<<"$out" >/dev/null || fail "finding not extracted after landing"
if jq -e . <<<"$out" >/dev/null 2>&1; then :; else fail "state JSON broken after adversarial content landed"; fi

[ "$FAILED" -eq 0 ] || exit 1
printf '%s\n' 'test-overview-state: PASS'
