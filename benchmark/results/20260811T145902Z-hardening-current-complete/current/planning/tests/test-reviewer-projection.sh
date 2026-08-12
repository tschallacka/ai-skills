#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_file="$repo_dir/planning/SKILL.md"
reviewer_file="$repo_dir/planning/REVIEWER.md"
expected_hash="$(sha256sum "$source_file" | awk '{print $1}')"
actual_hash="$(grep -o 'Source SHA-256: `[^`]*`' "$reviewer_file" | sed 's/.*: `//; s/`.*//')"
[ "$actual_hash" = "$expected_hash" ]
grep -Fq '## Generated sections' "$reviewer_file"
grep -Fq 'mandatory-review' "$reviewer_file"
grep -Fq 'bounded-context' "$reviewer_file"

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
planning_dir="$repo_dir/planning"
"$planning_dir/scripts/generate-reviewer.sh" "$planning_dir" "$tmp" >/dev/null
cmp -s "$tmp" "$reviewer_file"
printf '%s\n' 'test-reviewer-projection: PASS'
