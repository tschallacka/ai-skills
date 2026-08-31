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
planning_dir="$repo_dir/planning"
# shellcheck source=planning/tests/lib-test.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib-test.sh"

note_fail() { printf 'FAIL: %s\n' "$1" >&2; t_record "$1"; }

# REVIEWER.md is generated and never tracked (MAINTAINER.md section 2.16), so
# the contract is verified against fresh projections, not a committed copy:
# the pin, the markers, and two builds agreeing byte for byte.
tmp1="$(mktemp "${TMPDIR:-/tmp}/reviewer-projection.XXXXXX")"
tmp2="$(mktemp "${TMPDIR:-/tmp}/reviewer-projection.XXXXXX")"
trap 'rm -f "$tmp1" "$tmp2"' EXIT

if "$planning_dir/scripts/generate-reviewer.sh" "$planning_dir" "$tmp1" >/dev/null &&
   "$planning_dir/scripts/generate-reviewer.sh" "$planning_dir" "$tmp2" >/dev/null; then
    expected_hash="$(t_sha256 "$source_file")"
    actual_hash="$(grep -o 'Source SHA-256: `[^`]*`' "$tmp1" | sed 's/.*: `//; s/`.*//')"
    [ "$actual_hash" = "$expected_hash" ] \
        || note_fail "the projection pins $actual_hash but SKILL.md hashes to $expected_hash; run planning/scripts/generate-reviewer.sh"

    for marker in '## Generated sections' 'mandatory-review' 'bounded-context'; do
        grep -Fq "$marker" "$tmp1" || note_fail "the projection is missing '$marker'"
    done

    cmp -s "$tmp1" "$tmp2" \
        || note_fail 'two fresh projections differ; generation is not deterministic'
else
    note_fail 'generate-reviewer.sh could not produce a projection'
fi

if [ "$(t_failures)" -ne 0 ]; then
    printf 'test-reviewer-projection: %d failure(s).\n' "$(t_failures)" >&2
    exit 1
fi
printf 'test-reviewer-projection: PASS\n'
