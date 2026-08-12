#!/usr/bin/env bash
# Adversary-probe fixture version-compliance test.
#
# Guarantees the committed adversary-probe fixture (the reusable dummy plan
# used to probe fresh adversarial reviewers) stays in sync with the CURRENT
# planning-skill and gated-reader spec. There is NO backwards compatibility:
# when the spec changes, the fixture and its FIXTURE-VERSION marker must be
# updated in place or this test fails, so the probe never silently runs
# against a stale plan shape.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$root/tests/fixtures/adversary-probe"

[ -f "$fixture/FIXTURE-VERSION" ] || { echo "probe fixture missing: $fixture" >&2; exit 1; }

# Current spec versions.
current_skill="$(sed -n 's/^REVIEWER_PROFILE_VERSION="\(.*\)"$/\1/p' "$root/scripts/generate-reviewer.sh" | head -1)"
current_schema="$(sed -n 's/^context_schema_version=\([0-9]*\)$/\1/p' "$root/scripts/plan-context-lib.sh")"
current_generator="$(sed -n 's/^context_generator_version=\([0-9]*\)$/\1/p' "$root/scripts/plan-context-lib.sh")"
[ -n "$current_skill" ] && [ -n "$current_schema" ] && [ -n "$current_generator" ] || {
    echo "could not resolve current planning/reader spec versions" >&2; exit 1
}

# Fixture-declared versions.
declared_skill="$(sed -n 's/^skill_version=//p' "$fixture/FIXTURE-VERSION" | head -1)"
declared_schema="$(sed -n 's/^reader_schema_version=//p' "$fixture/FIXTURE-VERSION" | head -1)"
declared_generator="$(sed -n 's/^reader_generator_version=//p' "$fixture/FIXTURE-VERSION" | head -1)"

[ "$declared_skill" = "$current_skill" ] || {
    echo "probe fixture out of date: FIXTURE-VERSION skill_version=$declared_skill, current skill version=$current_skill (no backwards compatibility; update the fixture in place)" >&2
    exit 1
}
[ "$declared_schema" = "$current_schema" ] || {
    echo "probe fixture out of date: reader_schema_version=$declared_schema, current=$current_schema (no backwards compatibility; update the fixture in place)" >&2
    exit 1
}
[ "$declared_generator" = "$current_generator" ] || {
    echo "probe fixture out of date: reader_generator_version=$declared_generator, current=$current_generator (no backwards compatibility; update the fixture in place)" >&2
    exit 1
}

# Materialize a working copy (never mutate the committed fixture) and verify
# the CURRENT reader serves every entry id the probe relies on.
tmp="$(mktemp -d "${TMPDIR:-/tmp}/adversary-probe-fixture.XXXXXX")"
trap 'rm -rf -- "$tmp"' EXIT
cp -R "$fixture"/. "$tmp/plan"/
reader="$root/scripts/plan-context.sh"
bash "$reader" init --plan-dir "$tmp/plan" >/dev/null

for doc in plan inventory progress adversarial-review; do
    bash "$reader" read --plan-dir "$tmp/plan" --document "$doc" >/dev/null 2>&1 || {
        echo "fixture not compliant: reader does not serve --document $doc (current spec changed)" >&2
        exit 1
    }
done

for goal_dir in "$tmp/plan"/*/; do
    id="$(basename "$goal_dir")"
    [ "$id" = context ] && continue
    [ -f "$goal_dir/goal.md" ] || { echo "fixture missing goal.md for $id" >&2; exit 1; }
    bash "$reader" read --plan-dir "$tmp/plan" --document "goal:$id" >/dev/null 2>&1 || {
        echo "fixture not compliant: reader does not serve goal:$id" >&2; exit 1
    }
done

# The inventory schema must match what the reader parses (Goal col 9, Step
# col 10): each WNN row must resolve to an existing step file via the gate.
units="$(awk -F'|' 'function trim(v){gsub(/^[[:space:]]+|[[:space:]]+/,"",v); return v} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {print trim($2)}' "$tmp/plan/work-unit-inventory.md")"
[ -n "$units" ] || { echo "fixture inventory has no WNN rows" >&2; exit 1; }
for unit in $units; do
    bash "$reader" read --plan-dir "$tmp/plan" --unit "$unit" >/dev/null 2>&1 || {
        echo "fixture not compliant: unit $unit does not resolve (inventory Goal/Step columns wrong for the current reader)" >&2
        exit 1
    }
done

grep -Fq -- '- Status: pending' "$tmp/plan/adversarial-review.md" || {
    echo "fixture is not a reusable pending stub (adversarial-review.md was committed with a verdict)" >&2
    exit 1
}

printf 'Adversary-probe fixture is version-compliant (skill %s, reader schema %s, generator %s) and reusable through the gate.\n' \
    "$current_skill" "$current_schema" "$current_generator"
