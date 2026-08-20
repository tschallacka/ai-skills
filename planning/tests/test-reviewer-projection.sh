#!/usr/bin/env bash
# MODE: DEV
# test-reviewer-projection.sh — REVIEWER.md is a projection of SKILL.md, and it
# pins the whole-file SHA-256 of its source, so any SKILL.md edit must be
# followed by a regeneration.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_file="$repo_dir/planning/SKILL.md"
reviewer_file="$repo_dir/planning/REVIEWER.md"
planning_dir="$repo_dir/planning"
# shellcheck source=planning/tests/lib-test.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib-test.sh"

note_fail() { printf 'FAIL: %s\n' "$1" >&2; t_record "$1"; }

expected_hash="$(t_sha256 "$source_file")"
actual_hash="$(grep -o 'Source SHA-256: `[^`]*`' "$reviewer_file" | sed 's/.*: `//; s/`.*//')"
[ "$actual_hash" = "$expected_hash" ] \
    || note_fail "REVIEWER.md pins $actual_hash but SKILL.md hashes to $expected_hash; run planning/scripts/generate-reviewer.sh"

for marker in '## Generated sections' 'mandatory-review' 'bounded-context'; do
    grep -Fq "$marker" "$reviewer_file" || note_fail "REVIEWER.md is missing '$marker'"
done

tmp="$(mktemp "${TMPDIR:-/tmp}/reviewer-projection.XXXXXX")"
trap 'rm -f "$tmp"' EXIT
if "$planning_dir/scripts/generate-reviewer.sh" "$planning_dir" "$tmp" >/dev/null; then
    cmp -s "$tmp" "$reviewer_file" \
        || note_fail "REVIEWER.md differs from a fresh projection of SKILL.md; run planning/scripts/generate-reviewer.sh"
else
    note_fail 'generate-reviewer.sh could not produce a projection'
fi

if [ "$(t_failures)" -ne 0 ]; then
    printf 'test-reviewer-projection: %d failure(s).\n' "$(t_failures)" >&2
    exit 1
fi
printf 'test-reviewer-projection: PASS\n'
