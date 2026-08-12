#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
grep -Fq 'PROTOCOL_ID="reviewer-optimization-1.4.2"' "$root/setup-benchmark.sh"
grep -Fq 'protocol-metadata.json' "$root/setup-benchmark.sh"
grep -Fq 'distinct' "$root/analyzer-prompt.md"
grep -Fq '1.4.2 cohort' "$root/analyzer-prompt.md"
grep -Fq 'Legacy archives remain frozen' "$root/README.md"
grep -Fq 'copy_workspace_for_publication' "$root/setup-benchmark.sh"
grep -Fq -- "--exclude='.env'" "$root/setup-benchmark.sh"
grep -Fq -- "--exclude='*/.env'" "$root/setup-benchmark.sh"

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
