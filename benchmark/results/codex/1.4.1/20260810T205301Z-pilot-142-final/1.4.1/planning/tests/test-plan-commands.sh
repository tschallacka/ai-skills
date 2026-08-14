#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-command-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT
plan_dir="$temporary_root/status-sync"

if "$script_dir/update-plan-content.sh" paragraph "$plan_dir" plan -p 2.1: 'Legacy form' >/dev/null 2>&1; then
    echo 'The removed positional update form unexpectedly passed.' >&2
    exit 1
fi

"$script_dir/create-plan.sh" "$plan_dir" 'Status synchronization'
"$script_dir/add-goal.sh" "$plan_dir" 01-sync 'Synchronize status' \
    'Keep the review verdict and plan description aligned.'
"$script_dir/update-plan-content.sh" --testing-requirement "$plan_dir" 01-sync yes \
    'The status mutation and its validation have observable command output.'
"$script_dir/update-plan-content.sh" --goal-paragraph "$plan_dir" 01-sync 4.1 \
    'The goal keeps the review verdict and plan description aligned.'
"$script_dir/update-plan-content.sh" --goal-section "$plan_dir" 01-sync scope \
    -p 5.1: 'Update only the synchronized status fields.'
step_writer_output="$temporary_root/step-writer-output.log"
"$script_dir/add-work-unit.sh" "$plan_dir" W01 source planning/scripts/update-plan-content.sh \
    'review-status command' N/A 'Update both review status fields atomically.' '—' \
    01-sync 01-step-update-status >"$step_writer_output"
grep -Fq 'Reminder: goal 01-sync requires testing; continue with its test/proof step.' "$step_writer_output"
"$script_dir/add-work-unit.sh" "$plan_dir" W02 verification N/A 'validate-plan.sh' N/A \
    'Validate the status fields after synchronization.' W01 01-sync 02-step-validate-status
printf '# Verification: update status\n\n## Automated tests\n\nRun the status update command.\n' \
    > "$plan_dir/01-sync/steps/01-step-update-status-testing.md"
printf '# Verification: validate status\n\n## Automated tests\n\nRun the validator.\n' \
    > "$plan_dir/01-sync/steps/02-step-validate-status-testing.md"
step_update_output="$temporary_root/step-update-output.log"
"$script_dir/update-plan-content.sh" --step-section "$plan_dir" \
    01-sync/01-step-update-status objective \
    -p 4.1: 'Update both status fields atomically.' >"$step_update_output"
grep -Fq 'Reminder: testing instructions already exist at ' "$step_update_output"
"$script_dir/add-coverage.sh" "$plan_dir" 'Status fields are synchronized.' W01 \
    'The update command owns the mutation.'
"$script_dir/add-coverage.sh" "$plan_dir" 'The synchronized state is validated.' W02 \
    'The verification command proves the mutation.'

if "$script_dir/update-plan-content.sh" --description-section "$plan_dir" affected-areas \
    -p 7.1: 'Wrong section prefix.' >/dev/null 2>&1; then
    echo 'A mismatched paragraph section unexpectedly passed.' >&2
    exit 1
fi
"$script_dir/update-plan-content.sh" --description-section "$plan_dir" affected-areas \
    -p 6.1: 'Text containing -p is valid when quoted.'
grep -Fq 'Text containing -p is valid when quoted.' "$plan_dir/plan-description.md"
if "$script_dir/update-plan-content.sh" --description-section "$plan_dir" affected-areas \
    -p 6.1: 'Text containing § is reserved.' >/dev/null 2>&1; then
    echo 'A reserved paragraph marker unexpectedly passed.' >&2
    exit 1
fi

"$script_dir/update-plan-content.sh" --description-section "$plan_dir" affected-areas \
    -p 6.1: The plan description and review artifact are updated together. \
    -p 6.2: The validator reads matching status fields.
"$script_dir/update-plan-content.sh" --table-paragraph "$plan_dir" plan 6.1 2 \
    '"Name","Value"\n"Status","He said ""go"" and then ""stop"""'
grep -Fqx '| Name | Value |' "$plan_dir/plan-description.md"
grep -Fqx '| Status | He said "go" and then "stop" |' "$plan_dir/plan-description.md"
"$script_dir/update-plan-content.sh" --table-paragraph "$plan_dir" plan 6.1 2 \
    '"Name","Value"\n"Status","He said \"go\""'
grep -Fqx '| Status | He said "go" |' "$plan_dir/plan-description.md"
if "$script_dir/update-plan-content.sh" --table-paragraph "$plan_dir" plan 6.1 3 \
    'Name,Value\nStatus,approved' >/dev/null 2>&1; then
    echo 'A CSV table with the wrong column count unexpectedly passed.' >&2
    exit 1
fi
"$script_dir/update-plan-content.sh" --insert-after "$plan_dir" plan 6.1 \
    'Inserted after the table.'
grep -Fqx '§ 6.2' "$plan_dir/plan-description.md"
grep -Fqx '§ 6.3' "$plan_dir/plan-description.md"
"$script_dir/update-plan-content.sh" --insert-before "$plan_dir" plan 6.1 \
    'Inserted before the table.'
grep -Fqx '§ 6.4' "$plan_dir/plan-description.md"
grep -Fqx '§ 7.1' "$plan_dir/plan-description.md"
"$script_dir/update-plan-content.sh" -dp "$plan_dir" 6.2 \
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
"$script_dir/update-plan-content.sh" --decomposition-review "$plan_dir" completed

"$script_dir/create-adversarial-review.sh" "$plan_dir"
if "$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/pending-validation.log" 2>&1; then
    echo 'A pending review unexpectedly passed validation.' >&2
    exit 1
fi
grep -Fqx 'Plan validation failed with 2 error(s).' "$temporary_root/pending-validation.log"
"$script_dir/update-plan-content.sh" --review-section "$plan_dir" findings \
    -p 2.1: 'No unresolved findings remain after review.'
"$script_dir/update-plan-content.sh" -rp "$plan_dir" 2.1 \
    'No unresolved findings remain after the independent review.'
"$script_dir/update-plan-content.sh" --review-status "$plan_dir" approved
grep -Fqx -- '- Status: `✅ approved`' "$plan_dir/adversarial-review.md"
grep -Fqx -- '- Status: ✅ approved' "$plan_dir/plan-description.md"

"$script_dir/plan-content.sh" get "$plan_dir" unit:W01 json | grep -Fq '"id":"unit:W01"'
"$script_dir/plan-content.sh" blast-radius "$plan_dir" W01 json | grep -Fq '"downstream":["W02"]'
"$script_dir/validate-plan.sh" "$plan_dir"

missing_companion="$plan_dir/01-sync/steps/02-step-validate-status-testing.md"
mv "$missing_companion" "$missing_companion.missing"
if "$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/missing-companion.log" 2>&1; then
    echo 'A required testing companion unexpectedly passed validation when missing.' >&2
    exit 1
fi
grep -Fq "W02 requires testing instructions at $missing_companion" "$temporary_root/missing-companion.log"
mv "$missing_companion.missing" "$missing_companion"

"$script_dir/add-goal.sh" "$plan_dir" 02-research 'Research only' \
    'Record an untestable research result.'
"$script_dir/add-work-unit.sh" "$plan_dir" W03 discovery research-notes.md \
    'research scope' N/A 'Record bounded research findings.' '—' \
    02-research 01-step-research
"$script_dir/add-coverage.sh" "$plan_dir" 'Research findings are recorded.' W03 \
    'The discovery work unit owns the research record.'
"$script_dir/update-plan-content.sh" --testing-requirement "$plan_dir" 02-research no \
    'Research findings have no reproducible behavior to exercise.'
"$script_dir/validate-plan.sh" "$plan_dir" >/dev/null
"$script_dir/update-plan-content.sh" --testing-requirement "$plan_dir" 02-research yes \
    'This deliberately exercises the missing-test validation.'
if "$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/missing-test.log" 2>&1; then
    echo 'A goal marked as requiring testing unexpectedly passed without a test.' >&2
    exit 1
fi
grep -Fq '02-research declares testing is required but has no test or verification work unit' \
    "$temporary_root/missing-test.log"

printf 'Planning command regression test passed.\n'
