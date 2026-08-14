#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $(basename "$0") <plan-directory>" >&2
    exit 64
fi

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
    printf '| ID | Missing or over-broad item | Required plan change | Status |\n'
    printf '|---|---|---|---|\n'
    printf '| AR-01 | No finding recorded yet. | N/A | ✅ resolved |\n\n'
    printf '## Verdict\n\n'
    printf '%s\n' '- Status: `💤 pending`'
    printf '%s\n' '- Rationale: <why no unresolved work remains>'
} > "$temporary_file"
mv "$temporary_file" "$review_file"
trap - EXIT

echo "Created $review_file"
