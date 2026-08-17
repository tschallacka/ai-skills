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
# --field is deterministic: <plan-directory> <document-id> <field-label> <value>.
"$script_dir/update-plan-content.sh" --field "$plan_dir" plan 'Rationale' \
    'No user-visible component changes; command harness only.'
grep -Fq -- '- Rationale: No user-visible component changes; command harness only.' "$plan_dir/plan-description.md"
if "$script_dir/update-plan-content.sh" --field "$plan_dir" 'Rationale' 'x' >/dev/null 2>&1; then
    echo 'A --field without an explicit document-id unexpectedly passed.' >&2
    exit 1
fi
# --title is deterministic: <plan-directory> <document-id> <title>.
"$script_dir/update-plan-content.sh" --title "$plan_dir" plan 'Status synchronization v2'
grep -Fqx '# Plan: Status synchronization v2' "$plan_dir/plan-description.md"
if "$script_dir/update-plan-content.sh" --title "$plan_dir" 'Status' >/dev/null 2>&1; then
    echo 'A --title without an explicit document-id unexpectedly passed.' >&2
    exit 1
fi
# --table-paragraph is deterministic: <plan> <document-id> <N.N> <cols> <CSV>.
"$script_dir/update-plan-content.sh" --table-paragraph "$plan_dir" plan 5.1 2 \
    '"A","B"\n"1","2"'
grep -Fqx '| A | B |' "$plan_dir/plan-description.md"
grep -Fqx '| 1 | 2 |' "$plan_dir/plan-description.md"
if "$script_dir/update-plan-content.sh" --table-paragraph "$plan_dir" 5.1 2 'x' >/dev/null 2>&1; then
    echo 'A --table-paragraph without an explicit document-id unexpectedly passed.' >&2
    exit 1
fi
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
# update-adversarial-review: --help, stdin, and --file all work.
"$script_dir/update-adversarial-review.sh" --help >/dev/null 2>&1
printf 'ID,Missing or over-broad item,Required plan change,Status,Work unit\nAR-01,"x","y","✅ resolved",\n' | \
    "$script_dir/update-adversarial-review.sh" "$plan_dir" >/dev/null 2>&1
grep -Fq '| AR-01 |' "$plan_dir/adversarial-review.md"
printf 'ID,Missing,Required,Status,Work unit\nAR-02,a,b,✅ resolved,\n' > "$temporary_root/review.csv"
"$script_dir/update-adversarial-review.sh" "$plan_dir" --file "$temporary_root/review.csv" >/dev/null 2>&1
grep -Fq '| AR-02 |' "$plan_dir/adversarial-review.md"
# A pending review warns in normal mode but passes; --complete stays strict.
if ! "$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/pending-validation.log" 2>&1; then
    echo 'A pending review should validate with warnings only (not fail).' >&2
    exit 1
fi
grep -Fq 'WARN: Adversarial review is not approved' "$temporary_root/pending-validation.log"
if "$script_dir/validate-plan.sh" --complete "$plan_dir" >"$temporary_root/complete-pending.log" 2>&1; then
    echo '--complete mode accepted a pending review.' >&2
    exit 1
fi
grep -Fqx 'FAIL: Adversarial review is not approved' "$temporary_root/complete-pending.log"
"$script_dir/update-plan-content.sh" --review-section "$plan_dir" findings \
    -p 2.1: 'No unresolved findings remain after review.'
"$script_dir/update-plan-content.sh" -rp "$plan_dir" 2.1 \
    'No unresolved findings remain after the independent review.'
"$script_dir/update-plan-content.sh" --review-status "$plan_dir" approved
grep -Fqx -- '- Status: `✅ approved`' "$plan_dir/adversarial-review.md"
grep -Fqx -- '- Status: ✅ approved' "$plan_dir/plan-description.md"

"$script_dir/plan-content.sh" get "$plan_dir" unit:W01 json | grep -Fq '"id":"unit:W01"'
# plan-content.sh get is deterministic: <plan> <document-id> [format].
"$script_dir/plan-content.sh" get "$plan_dir" plan json | grep -Fq '"id":"plan"'
"$script_dir/plan-content.sh" get "$plan_dir" plan | grep -Fq '# Plan: '
if "$script_dir/plan-content.sh" get "$plan_dir" json >/dev/null 2>&1; then
    echo 'plan-content get without an explicit document-id unexpectedly passed.' >&2
    exit 1
fi
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

profile_source="$temporary_root/SKILL.md"
profile_output="$temporary_root/REVIEWER.md"
cp "$script_dir/../SKILL.md" "$profile_source"
generated_output="$($script_dir/generate-reviewer.sh "$temporary_root" "$profile_output")"
grep -Fq 'profile contract: `1.4.2`' "$profile_output"
grep -Eq '^> Source SHA-256: `[0-9a-f]{64}`$' "$profile_output"
grep -Fq '## 3. Mandatory classification and independent review' "$profile_output"
grep -Fq '## 4.1 Bounded context and portable plan storage' "$profile_output"
grep -Fq 'generated ' <<<"$generated_output"

cp "$profile_output" "$temporary_root/REVIEWER.before-drift.md"
printf '\n' >> "$profile_source"
"$script_dir/generate-reviewer.sh" "$temporary_root" "$profile_output" >/dev/null
if cmp -s "$temporary_root/REVIEWER.before-drift.md" "$profile_output"; then
    echo 'A changed source unexpectedly produced an unchanged reviewer profile.' >&2
    exit 1
fi

missing_dir="$temporary_root/missing-section"
mkdir -p "$missing_dir"
missing_source="$missing_dir/SKILL.md"
cp "$script_dir/../SKILL.md" "$missing_source"
sed -i '/REVIEWER_SECTION:END bounded-context/d' "$missing_source"
if "$script_dir/generate-reviewer.sh" "$missing_dir" "$temporary_root/missing.md" >/dev/null 2>&1; then
    echo 'A missing reviewer section unexpectedly passed generation.' >&2
    exit 1
fi

empty_dir="$temporary_root/empty-section"
mkdir -p "$empty_dir"
empty_source="$empty_dir/SKILL.md"
cp "$script_dir/../SKILL.md" "$empty_source"
sed -i '/REVIEWER_SECTION:START bounded-context/,/REVIEWER_SECTION:END bounded-context/{ /START/d; /END/!d; }' "$empty_source"
if "$script_dir/generate-reviewer.sh" "$empty_dir" "$temporary_root/empty.md" >/dev/null 2>&1; then
    echo 'An empty reviewer section unexpectedly passed generation.' >&2
    exit 1
fi

# ---- proactive helper behaviors (regression) ----
# -dp auto-creates the next sequential paragraph and reports it.
"$script_dir/update-plan-content.sh" -dp "$plan_dir" 2.2 'Auto-created paragraph.'
grep -Fq '§ 2.2' "$plan_dir/plan-description.md"
# Invalid section ids list the valid ids (actionable error).
if "$script_dir/update-plan-content.sh" -ds "$plan_dir" bogus-section -p 5.1: x >"$temporary_root/bogus-sec.log" 2>&1; then
    echo 'An invalid section id unexpectedly passed.' >&2
    exit 1
fi
grep -Fq 'Valid plan section ids' "$temporary_root/bogus-sec.log"
# remove-work-unit --help exits 0.
"$script_dir/remove-work-unit.sh" --help >/dev/null 2>&1
# Removing W02 reconciles inventory, coverage, owned work units, and progress.
"$script_dir/remove-work-unit.sh" "$plan_dir" W02 >/dev/null 2>&1
if grep -Fq '| W02 |' "$plan_dir/work-unit-inventory.md"; then
    echo 'Removed work unit W02 still present in the inventory.' >&2
    exit 1
fi
if grep -Fq 'W02' "$plan_dir/01-sync/goal.md"; then
    echo 'Removed work unit W02 still referenced in goal.md owned work units.' >&2
    exit 1
fi
if grep -Fq '| W02 |' "$plan_dir/01-sync/progress.md"; then
    echo 'Removed work unit W02 still present in the goal progress tracker.' >&2
    exit 1
fi
# ---- defect-report hardening (regression) ----
# P0-1: flag-shaped content in a paragraph edit is rejected with the section hint.
if "$script_dir/update-plan-content.sh" -gp "$plan_dir" 01-sync 4.1 '-p 2.1: flag-shaped' \
    >"$temporary_root/swallowed-flag.log" 2>&1; then
    echo 'Flag-shaped paragraph content unexpectedly passed.' >&2
    exit 1
fi
grep -Fq 'flag-shaped' "$temporary_root/swallowed-flag.log"
# P0-2: plan-content.sh find locates a literal string; ambiguous hits exit 1.
"$script_dir/plan-content.sh" find "$plan_dir" '01-step-update-status' --in steps
if "$script_dir/plan-content.sh" find "$plan_dir" 'Status' >"$temporary_root/find-multi.log" 2>&1; then
    echo 'An ambiguous find unexpectedly exited 0.' >&2
    exit 1
fi
grep -Fq 'narrow the pattern or scope' "$temporary_root/find-multi.log"
if "$script_dir/plan-content.sh" find "$plan_dir" 'no-such-string-xyz' >"$temporary_root/find-zero.log" 2>&1; then
    echo 'A find with no matches unexpectedly exited 0.' >&2
    exit 1
fi
# P1-2: append-paragraph adds the next free paragraph number in the section.
"$script_dir/update-plan-content.sh" -ap "$plan_dir" step:01-sync/01-step-update-status objective \
    'Appended objective paragraph.'
grep -Fq '§ 4.2' "$plan_dir/01-sync/steps/01-step-update-status.md"
# P1-1: mutating helpers commit a pre-mutation snapshot in the plan git repo.
git -C "$plan_dir" log --oneline > "$temporary_root/git-log.txt"
grep -Fq 'snapshot before' "$temporary_root/git-log.txt"
# P2-1: the validator rejects helper-flag-shaped text in narrative documents.
printf '\n§ 5.9\nrun update-plan-content.sh -dp 2.3: x\n' >> "$plan_dir/plan-description.md"
if "$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/flag-shaped-validation.log" 2>&1; then
    echo 'Flag-shaped text in a narrative paragraph unexpectedly validated.' >&2
    exit 1
fi
grep -Fq 'helper-flag-shaped text' "$temporary_root/flag-shaped-validation.log"
git -C "$plan_dir" checkout -- plan-description.md
# §5c: update-work-unit.sh amends type, depends-on, and description in place;
# coverage rows are never touched.
"$script_dir/update-work-unit.sh" "$plan_dir" W03 --type docs --depends-on W01 \
    --description 'Record bounded research findings in the archive.'
grep -Fq '| W03 | docs |' "$plan_dir/work-unit-inventory.md"
grep -Fq '| W03 | docs |' "$plan_dir/work-unit-inventory.md"
grep -Fq 'Record bounded research findings in the archive. | W01 | 02-research |' "$plan_dir/work-unit-inventory.md"
grep -Fq -- '- Type: `docs`' "$plan_dir/02-research/steps/01-step-research.md"
grep -Fq 'Record bounded research findings in the archive.' "$plan_dir/02-research/steps/01-step-research.md"
grep -Fq '| Research findings are recorded. | W03 |' "$plan_dir/work-unit-inventory.md"
# P2-2: removal refuses to cascade dependency pruning without --confirm-cascade.
if "$script_dir/remove-work-unit.sh" "$plan_dir" W01 >"$temporary_root/cascade-refused.log" 2>&1; then
    echo 'Removal with dependents unexpectedly passed without --confirm-cascade.' >&2
    exit 1
fi
grep -Fq 'still list it in Depends-on' "$temporary_root/cascade-refused.log"
"$script_dir/remove-work-unit.sh" "$plan_dir" W01 --confirm-cascade >/dev/null 2>&1
if grep -Fq '| W01 |' "$plan_dir/work-unit-inventory.md"; then
    echo 'Work unit W01 still present after a confirmed cascade removal.' >&2
    exit 1
fi
printf 'Planning command regression test passed.\n'
