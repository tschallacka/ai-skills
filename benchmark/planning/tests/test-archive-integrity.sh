#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The generated-case source spans setup-benchmark.sh *and* the extracted
# benchmark/planning/case/*.sh, so harness-wide assertions search both, matching
# the harness_grep idiom in test-safeguards.sh.
harness_sources=("$root/setup-benchmark.sh")
for case_source in "$root/case"/*.sh; do
    [ -f "$case_source" ] && harness_sources+=("$case_source")
done
harness_grep() { grep -Fq -- "$1" "${harness_sources[@]}"; }
harness_grep 'PROTOCOL_ID="reviewer-optimization-1.4.2"'
harness_grep 'protocol-metadata.json'
grep -Fq 'distinct' "$root/analyzer-prompt.md"
grep -Fq '1.4.2 cohort' "$root/analyzer-prompt.md"
grep -Fq 'Legacy archives remain frozen' "$root/README.md"
harness_grep 'copy_workspace_for_publication'
harness_grep "--exclude='.env'"
harness_grep "--exclude='*/.env'"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/source/plan/nested" "$tmp/target"
printf '%s\n' evidence > "$tmp/source/plan/plan-description.md"
printf '%s\n' secret > "$tmp/source/.env"
printf '%s\n' secret > "$tmp/source/plan/.env"
printf '%s\n' secret > "$tmp/source/plan/nested/.env.tmp.secret"
tar -C "$tmp/source" \
    --exclude='.env' --exclude='.env.tmp.*' \
    --exclude='*/.env' --exclude='*/.env.tmp.*' \
    -cf - . | tar -C "$tmp/target" -xf -
[ -f "$tmp/target/plan/plan-description.md" ]
[ ! -e "$tmp/target/.env" ]
[ ! -e "$tmp/target/plan/.env" ]
[ ! -e "$tmp/target/plan/nested/.env.tmp.secret" ]
printf 'Archive integrity contract tests passed.\n'
