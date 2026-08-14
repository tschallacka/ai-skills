#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-command-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT
plan_dir="$temporary_root/status-sync"

"$script_dir/create-plan.sh" "$plan_dir" 'Status synchronization'
"$script_dir/add-goal.sh" "$plan_dir" 01-sync 'Synchronize status' \
    'Keep the review verdict and plan description aligned.'
"$script_dir/add-work-unit.sh" "$plan_dir" W01 source planning/scripts/update-plan-content.sh \
    'review-status command' N/A 'Update both review status fields atomically.' '—' \
    01-sync 01-step-update-status
"$script_dir/add-work-unit.sh" "$plan_dir" W02 verification N/A 'validate-plan.sh' N/A \
    'Validate the status fields after synchronization.' W01 01-sync 02-step-validate-status
"$script_dir/add-coverage.sh" "$plan_dir" 'Status fields are synchronized.' W01 \
    'The update command owns the mutation.'
"$script_dir/add-coverage.sh" "$plan_dir" 'The synchronized state is validated.' W02 \
    'The verification command proves the mutation.'

if "$script_dir/update-plan-content.sh" section "$plan_dir" plan affected-areas \
    -p 7.1: 'Wrong section prefix.' >/dev/null 2>&1; then
    echo 'A mismatched paragraph section unexpectedly passed.' >&2
    exit 1
fi
if "$script_dir/update-plan-content.sh" section "$plan_dir" plan affected-areas \
    -p 6.1: 'Text containing -p is reserved.' >/dev/null 2>&1; then
    echo 'A reserved paragraph token unexpectedly passed.' >&2
    exit 1
fi

"$script_dir/update-plan-content.sh" section "$plan_dir" plan affected-areas \
    -p 6.1: The plan description and review artifact are updated together. \
    -p 6.2: The validator reads matching status fields.
"$script_dir/update-plan-content.sh" paragraph "$plan_dir" plan -p 6.2: \
    'The validator reads matching status fields from both documents.'
grep -Fqx '§ 6.1' "$plan_dir/plan-description.md"
grep -Fqx '§ 6.2' "$plan_dir/plan-description.md"

"$script_dir/create-ui-validation.sh" "$plan_dir" 'Serve index.html from a local static HTTP server.'
"$script_dir/add-ui-story.sh" "$plan_dir" US-01 'A visitor sees the initial button.' \
    'Open the page and click the visible button.' 'Mouse click on the visible button.' \
    'The requested result is visible.' W01,W02
"$script_dir/configure-ui-story-cache.sh" "$plan_dir" US-01 \
    'The locally served page is open and shows its initial button.' \
    'Mouse click on the visible button.' 'The visible button.' \
    'The requested result is visible.' '2 s'
"$script_dir/update-plan-content.sh" decomposition-review "$plan_dir" completed

"$script_dir/create-adversarial-review.sh" "$plan_dir"
if "$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/pending-validation.log" 2>&1; then
    echo 'A pending review unexpectedly passed validation.' >&2
    exit 1
fi
grep -Fqx 'Plan validation failed with 2 error(s).' "$temporary_root/pending-validation.log"
"$script_dir/update-plan-content.sh" section "$plan_dir" review findings \
    -p 2.1: 'No unresolved findings remain after review.'
"$script_dir/update-plan-content.sh" review-status "$plan_dir" approved
grep -Fqx -- '- Status: `✅ approved`' "$plan_dir/adversarial-review.md"
grep -Fqx -- '- Status: ✅ approved' "$plan_dir/plan-description.md"

"$script_dir/plan-content.sh" get "$plan_dir" unit:W01 json | grep -Fq '"id":"unit:W01"'
"$script_dir/plan-content.sh" blast-radius "$plan_dir" W01 json | grep -Fq '"downstream":["W02"]'
"$script_dir/validate-plan.sh" "$plan_dir"

printf 'Planning command regression test passed.\n'
