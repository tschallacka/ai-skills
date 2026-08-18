#!/usr/bin/env bash
# create-adversarial-review.sh — seed a plan's adversarial-review.md with the
# Review scope, the 5-column Findings table, and a pending Verdict.
#
# The seeded AR-01 row is the empty state of the table, and the Findings table
# itself is the insertion anchor every consumer agrees on: add-adversarial-finding.sh
# appends after its last row, update-adversarial-review.sh rewrites the span from
# "## Findings" to "## Verdict", and mint-fix-keys.sh reads the same span.
#
# Usage:
#   create-adversarial-review.sh <plan-directory>
#   create-adversarial-review.sh --help

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <plan-directory>
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in -h|--help) usage 0 ;; esac
[ "$#" -eq 1 ] || usage

plan_dir="$1"
review_file="$plan_dir/adversarial-review.md"
if [ ! -d "$plan_dir" ]; then
    echo "Plan directory not found: $plan_dir" >&2
    exit 66
fi
if [ -e "$review_file" ]; then
    echo "Adversarial review already exists: $review_file" >&2
    exit 73
fi

temporary_file="${review_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
{
    printf '# Adversarial review: %s\n\n' "$(basename "$plan_dir")"
    printf '## Review scope\n\n'
    printf '§ 1.1\n'
    printf '%s\n' '- Request: <verbatim or precise summary>'
    printf '%s\n\n' '- Repository/context inspected: <what was checked>'
    printf '## Findings\n\n'
    printf '| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n'
    printf '|---|---|---|---|---|\n'
    printf '| AR-01 | No finding recorded yet. | N/A | ✅ resolved | N/A |\n\n'
    printf '## Verdict\n\n'
    printf '%s\n' '- Status: `💤 pending`'
    printf '%s\n' '- Rationale: <why no unresolved work remains>'
} > "$temporary_file"
mv "$temporary_file" "$review_file"

printf 'Created %s\n' "$review_file"
