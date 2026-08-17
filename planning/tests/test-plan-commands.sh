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
# -t is the short form of --title (the accepted -t flag must be covered too).
"$script_dir/update-plan-content.sh" -t "$plan_dir" plan 'Status synchronization v3'
grep -Fqx '# Plan: Status synchronization v3' "$plan_dir/plan-description.md"
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
# 02-research owns exactly one work unit, so it needs the goal-size exception
# section with a reason (add-goal no longer emits the placeholder).
"$script_dir/update-plan-content.sh" -gs "$plan_dir" 02-research goal-size-exception \
    -p 11.1: 'Genuinely standalone discovery outcome, exempted under the single-unit goal gate.'
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
# ---- report 3: propagation, coverage/stories readability, history, --replace,
#      stale sweep, verify-target, diff ----
# Re-establish W01/W02 under a fresh goal for the propagation/grader checks
# (the earlier W01/W02 were removed by the P2-2 cascade test).
"$script_dir/add-goal.sh" "$plan_dir" 03-wire 'Wire history' \
    'Wire the order history surface.' >/dev/null 2>&1
"$script_dir/add-work-unit.sh" "$plan_dir" W10 markup \
    app/design/frontend/FakeTheme/templates/order/history.phtml '#order_history' N/A \
    'Render the order history block.' '—' 03-wire 01-step-history >/dev/null 2>&1
"$script_dir/add-work-unit.sh" "$plan_dir" W11 verification N/A 'verify-history.sh' N/A \
    'Verify the order history block renders.' W10 03-wire 02-step-verify >/dev/null 2>&1
"$script_dir/update-plan-content.sh" --testing-requirement "$plan_dir" 03-wire yes \
    'The render and its verification have observable output.' >/dev/null 2>&1
printf '# Verification: history\n\n## Automated tests\n\nRun the render check.\n' \
    > "$plan_dir/03-wire/steps/01-step-history-testing.md"
printf '# Verification: verify\n\n## Automated tests\n\nRun the verifier.\n' \
    > "$plan_dir/03-wire/steps/02-step-verify-testing.md"
"$script_dir/add-coverage.sh" "$plan_dir" 'Order history renders.' W10 'covers render' >/dev/null 2>&1
"$script_dir/add-coverage.sh" "$plan_dir" 'Order history is verified.' W11 'grader' >/dev/null 2>&1
# coverage and stories are readable document ids for get.
"$script_dir/plan-content.sh" get "$plan_dir" coverage | grep -Fq '## Definition-of-done coverage'
"$script_dir/plan-content.sh" get "$plan_dir" stories | grep -Fq 'UI user stories'
# find --in coverage scopes to the Definition-of-done coverage rows.
"$script_dir/plan-content.sh" find "$plan_dir" 'Order history renders.' --in coverage >/dev/null
# add-coverage --replace collapses duplicate coverage rows for the same outcome.
"$script_dir/add-coverage.sh" "$plan_dir" 'Duplicate-proof outcome.' W10 'first' >/dev/null 2>&1
"$script_dir/add-coverage.sh" "$plan_dir" 'Duplicate-proof outcome.' W10 'second' --replace >/dev/null 2>&1
if [ "$(grep -c '| Duplicate-proof outcome.' "$plan_dir/work-unit-inventory.md")" -ne 1 ]; then
    echo 'add-coverage --replace did not collapse duplicate coverage rows.' >&2
    exit 1
fi
grep -Fq '| Duplicate-proof outcome. | W10 | second |' "$plan_dir/work-unit-inventory.md"
# update-adversarial-review --cycle archives the PRIOR findings table into
# history; the new rows land in the live table and are archived by the next cycle.
printf 'AR-20,old gap,change it,✅ resolved,W10\n' | "$script_dir/update-adversarial-review.sh" "$plan_dir" --cycle 7 >/dev/null 2>&1
grep -Fq '## Cycle 7' "$plan_dir/adversarial-review-history.md"
printf 'AR-21,new gap,change it,✅ resolved,W10\n' | "$script_dir/update-adversarial-review.sh" "$plan_dir" --cycle 8 >/dev/null 2>&1
grep -Fq '## Cycle 8' "$plan_dir/adversarial-review-history.md"
# cycle 8 archived the cycle-7 table, which held AR-20.
grep -Fq '| AR-20 |' "$plan_dir/adversarial-review-history.md"
grep -Fq '## Cycle 7' "$plan_dir/adversarial-review-history.md"
# stale sweep: a phrase in an unmarked paragraph fails; with a history marker
# the stale check itself passes (unrelated later-plan failures are ignored here).
# The phrase is unique to the step objective so it does not collide with the
# goal owned-unit roster.
"$script_dir/update-plan-content.sh" -sp "$plan_dir" 03-wire/01-step-history 4.1 \
    'The rows must still render after the source retarget.' >/dev/null 2>&1
printf 'rows must still render after the source retarget\n' > "$temporary_root/stale.txt"
if "$script_dir/validate-plan.sh" --stale "$temporary_root/stale.txt" "$plan_dir" >"$temporary_root/stale-fail.log" 2>&1; then
    echo '--stale accepted a phrase in an unmarked paragraph.' >&2
    exit 1
fi
grep -Fq 'stale phrase' "$temporary_root/stale-fail.log"
"$script_dir/update-plan-content.sh" -sp "$plan_dir" 03-wire/01-step-history 4.1 \
    'The rows must still render after the source retarget (previously the rows must still render after the source retarget).' >/dev/null 2>&1
"$script_dir/validate-plan.sh" --stale "$temporary_root/stale.txt" "$plan_dir" >"$temporary_root/stale-pass.log" 2>&1 || true
if grep -Fq 'stale phrase' "$temporary_root/stale-pass.log"; then
    echo '--stale flagged a phrase inside a history-marked paragraph.' >&2
    exit 1
fi
# --stale default ships a bundled case-count list and sweeps companions.
printf '# Verification: history\n\n## Automated tests\n\nCheck all four states emit a row.\n' \
    > "$plan_dir/03-wire/steps/01-step-history-testing.md"
if "$script_dir/validate-plan.sh" --stale default "$plan_dir" >"$temporary_root/stale-default.log" 2>&1; then
    echo '--stale default missed a case-count phrase in a companion.' >&2
    exit 1
fi
grep -Fq 'stale phrase' "$temporary_root/stale-default.log"
grep -Fq '01-step-history-testing' "$temporary_root/stale-default.log"
# propagation: a grader that names a unit it does not depend on fails.
"$script_dir/update-plan-content.sh" -sp "$plan_dir" 03-wire/02-step-verify 4.1 \
    'Verify W10 renders the order history surface.' >/dev/null 2>&1
"$script_dir/update-work-unit.sh" "$plan_dir" W11 --depends-on '—' >/dev/null 2>&1
if "$script_dir/validate-plan.sh" --propagation "$plan_dir" >"$temporary_root/prop.log" 2>&1; then
    echo '--propagation accepted a grader with no dependency edge.' >&2
    exit 1
fi
grep -Fq 'verification unit that grades' "$temporary_root/prop.log"
"$script_dir/update-work-unit.sh" "$plan_dir" W11 --depends-on 'W10' >/dev/null 2>&1
# report 9: propagation is on by default; short-form seams, template ids, and
# ::class are NOT ownership violations (the plan never names them as change
# targets); a cross-plan WNN reference is prose, not a typo.
"$script_dir/update-plan-content.sh" -ss "$plan_dir" 03-wire/01-step-history instructions \
    -p 5.1: 'Call AbstractItems::getColumnHtml() and QuoteManagement::submit — seams to record the boundary.' \
    -p 5.2: 'Reference Magento_Weee::email/items/price/row.phtml and Foo::class.' \
    -p 5.3: 'The extended-rendering plan'"'"'s W99 and W98, explicitly outside this plan'"'"'s scope.' >/dev/null 2>&1
if "$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/report9.log" 2>&1; then
    :
fi
if grep -qE 'AbstractItems|QuoteManagement|Magento_Weee|::class|W99|W98' "$temporary_root/report9.log"; then
    echo 'report 9: a seam, template id, ::class, or cross-plan reference was flagged.' >&2
    exit 1
fi
# report 10: --delete-paragraph of the last paragraph in a section must not
# destroy the following heading (acceptance criteria content stays its own).
"$script_dir/update-plan-content.sh" -ss "$plan_dir" 03-wire/01-step-history instructions \
    -p 5.1: 'Only instruction.' >/dev/null 2>&1
"$script_dir/update-plan-content.sh" -ss "$plan_dir" 03-wire/01-step-history acceptance-criteria \
    -p 6.1: 'Only criterion.' >/dev/null 2>&1
before_headings="$(grep -c '^## ' "$plan_dir/03-wire/steps/01-step-history.md")"
"$script_dir/update-plan-content.sh" --delete-paragraph "$plan_dir" step:03-wire/01-step-history 5.1 >/dev/null 2>&1
if [ "$(grep -c '^## ' "$plan_dir/03-wire/steps/01-step-history.md")" -ne "$before_headings" ]; then
    echo 'report 10: deleting the last instruction paragraph removed a section heading.' >&2
    exit 1
fi
grep -Fq '## Acceptance criteria' "$plan_dir/03-wire/steps/01-step-history.md"
grep -Fq 'Only criterion.' "$plan_dir/03-wire/steps/01-step-history.md"
# report 10: fixes.md, fix-keys.json, approval.json are readable doc ids; find
# --in inventory is an alias for --in units.
printf 'AR-01\tW10\tkey\n' > "$plan_dir/fixes.md"
printf '{"session_id":"s1","keys":{}}\n' > "$plan_dir/fix-keys.json"
printf '{"status":"pending"}\n' > "$plan_dir/approval.json"
"$script_dir/plan-content.sh" get "$plan_dir" fixes | grep -Fq 'AR-01'
"$script_dir/plan-content.sh" get "$plan_dir" fix-keys | grep -Fq 'session_id'
"$script_dir/plan-content.sh" get "$plan_dir" approval | grep -Fq 'pending'
"$script_dir/plan-content.sh" find "$plan_dir" '01-step-history' --in inventory | grep -Fq 'unit:'
# report 12: roster↔inventory set check — a goal §9.x roster that omits an
# inventory-assigned unit is a propagation fail.
git -C "$plan_dir" checkout -- 03-wire/goal.md 2>/dev/null || true
python3 - "$plan_dir/03-wire/goal.md" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
for block in ("§ 9.2\n`W11` — Verify the order history block renders.\n",):
    s = s.replace(block, "")
open(p, "w").write(s)
PY
if "$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/roster.log" 2>&1; then
    echo 'report 12: roster omitting an assigned unit validated clean.' >&2
    exit 1
fi
grep -Fq 'roster omits W11' "$temporary_root/roster.log"
git -C "$plan_dir" checkout -- 03-wire/goal.md 2>/dev/null || true
# report 13: a summary §9.1 that lists all units bare before the em-dash (with
# a cross-plan W99 after it) plus only ONE per-unit blurb is a valid roster —
# the per-unit blurbs are NOT guaranteed to exist for every unit. Must not FAIL.
python3 - "$plan_dir/03-wire/goal.md" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
start = s.index('## Owned work units')
end = s.index('## Goal-size exception')
repl = ("## Owned work units\n\n"
        "§ 9.1\n"
        "W10, W11, in that order as steps 01 to 02 — both units, including the extended-rendering plan's W99\n\n"
        "## Testing requirement\n\n"
        "| Test required | Rationale |\n|---|---|\n"
        "| yes | observable |\n\n"
        "§ 9.2\n`W10` — Render the order history block.\n")
s = s[:start] + repl + s[end:]
open(p, 'w').write(s)
PY
"$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/roster13.log" 2>&1 || true
if grep -qE 'roster (omits|lists)' "$temporary_root/roster13.log"; then
    echo 'report 13: a summary-roster goal with partial blurbs and a cross-plan ref was flagged.' >&2
    exit 1
fi
# report 15 §4: a literal multi-word <...> placeholder in a GENERATED artifact
# (progress tracker) is a validator FAIL, even when the rest of the plan is fine.
printf '\n## Work units\n\n| Goalname | Description | Status |\n|---|---|---|\n| 03-wire | <short description> | ok |\n' >> "$plan_dir/progress.md"
if "$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/placeholder.log" 2>&1; then
    echo 'report 15: a generated artifact placeholder validated clean.' >&2
    exit 1
fi
grep -Fq 'unregistered/stale placeholder' "$temporary_root/placeholder.log"
git -C "$plan_dir" checkout -- progress.md 2>/dev/null || true
# A REGISTERED authoring placeholder (e.g. <definition of done>) is allowed: the
# registry is the allowlist, and only unregistered/stale placeholders fail.
printf '\n## Work units\n\n| Goalname | Description | Status |\n|---|---|---|\n| 03-wire | <definition of done> | ok |\n' >> "$plan_dir/progress.md"
"$script_dir/validate-plan.sh" "$plan_dir" >"$temporary_root/placeholder-ok.log" 2>&1 || true
if grep -Fq 'unregistered/stale placeholder' "$temporary_root/placeholder-ok.log"; then
    echo 'report 15: a registered authoring placeholder was flagged.' >&2
    cat "$temporary_root/placeholder-ok.log" >&2
    exit 1
fi
git -C "$plan_dir" checkout -- progress.md 2>/dev/null || true
# retargeting a unit lists the verification units that grade it.
retarget_output="$("$script_dir/update-work-unit.sh" "$plan_dir" W10 '#order_history' app/design/frontend/FakeTheme/templates/order/history.phtml 2>&1)"
printf '%s\n' "$retarget_output" | grep -Fq 're-read its grader(s) W11' || {
    echo 'retargeting did not list the grading verification unit.' >&2
    exit 1
}
# verify-target: a markup unit whose block is removed by a layout fails.
fake_repo="$temporary_root/fakerepo"
mkdir -p "$fake_repo/app/code/Fake/Module/view/frontend/layout" \
    "$fake_repo/app/design/frontend/FakeTheme/templates/order"
printf 'rendered template\n' > "$fake_repo/app/design/frontend/FakeTheme/templates/order/history.phtml"
if "$script_dir/verify-target.sh" "$plan_dir" W10 --repo "$fake_repo" >"$temporary_root/verify-target-ok.log" 2>&1; then
    :
else
    echo "verify-target passed on a reachable target." >&2
    cat "$temporary_root/verify-target-ok.log" >&2
    exit 1
fi
grep -Fq 'PASS' "$temporary_root/verify-target-ok.log"
printf '<layout><referenceBlock name="order_history" remove="true"/></layout>\n' \
    > "$fake_repo/app/code/Fake/Module/view/frontend/layout/catalog.xml"
if "$script_dir/verify-target.sh" "$plan_dir" W10 --repo "$fake_repo" >"$temporary_root/verify-target-fail.log" 2>&1; then
    echo 'verify-target accepted a target whose block is removed by a layout.' >&2
    cat "$temporary_root/verify-target-fail.log" >&2
    exit 1
fi
grep -Fq 'a layout removes block' "$temporary_root/verify-target-fail.log"
# plan-content.sh diff lists documents changed since a git ref. Mutating
# helpers snapshot (commit) their own change, so capture the ref BEFORE the
# mutation and diff against it.
git -C "$plan_dir" add -A -- . >/dev/null 2>&1
git -C "$plan_dir" -c user.name=plan-skill -c user.email=plan-skill@localhost \
    commit -q -m 'before diff' 2>/dev/null || true
before_ref="$(git -C "$plan_dir" rev-parse HEAD)"
"$script_dir/update-plan-content.sh" -sp "$plan_dir" 03-wire/01-step-history 4.1 \
    'The rows must still render after the diff baseline.' >/dev/null 2>&1
"$script_dir/plan-content.sh" diff "$plan_dir" "$before_ref" | grep -Fq '03-wire/steps/01-step-history.md'
"$script_dir/plan-content.sh" diff "$plan_dir" "$before_ref" | grep -Fq '§ 4.1'
# ---- report 5: companion find scope, --delete-paragraph, --scope, story
#      placeholder-free cache, goal-size placeholder omission ----
# find reaches the *-testing.md companions via --in testing and --in all.
"$script_dir/create-step-testing.sh" "$plan_dir/03-wire" 01-step-history \
    'Verify the rows still render.\nRecord the observed result.' --overwrite >/dev/null 2>&1
"$script_dir/plan-content.sh" find "$plan_dir" 'Verify the rows still render' --in testing \
    | grep -Fq 'step:03-wire/01-step-history-testing'
# --delete-paragraph removes one paragraph and renumbers the rest of the section.
"$script_dir/update-plan-content.sh" -ss "$plan_dir" 03-wire/01-step-history instructions \
    -p 5.1: 'Instruction one.' -p 5.2: 'Instruction two.' -p 5.3: 'Instruction three.' >/dev/null 2>&1
"$script_dir/update-plan-content.sh" --delete-paragraph "$plan_dir" step:03-wire/01-step-history 5.2 >/dev/null 2>&1
if grep -Fqx '§ 5.3' "$plan_dir/03-wire/steps/01-step-history.md"; then
    echo '--delete-paragraph did not renumber the following paragraph.' >&2
    exit 1
fi
grep -Fqx '§ 5.2' "$plan_dir/03-wire/steps/01-step-history.md"
grep -Fq 'Instruction three.' "$plan_dir/03-wire/steps/01-step-history.md"
# --scope is the flag form of the scope positional; ' and ' scope text passes.
"$script_dir/update-work-unit.sh" "$plan_dir" W10 --scope \
    'Renderer::render() and the History value object, both new in this file' >/dev/null 2>&1
grep -Fq 'Renderer::render() and the History value object' "$plan_dir/work-unit-inventory.md"
# add-goal emits the Goal-size exception heading empty, never the placeholder text.
"$script_dir/add-goal.sh" "$plan_dir" 04-clean 'Clean' 'No placeholder.' >/dev/null 2>&1
if grep -Fq '<required only when this goal has one permitted work unit>' "$plan_dir/04-clean/goal.md"; then
    echo 'add-goal still emits the goal-size placeholder text.' >&2
    exit 1
fi
# add-ui-story leaves a cache that validates (no <...> template placeholders).
if [ -f "$plan_dir/ui-story-runs/US-01.md" ]; then
    if grep -Eq '<[^>]+>' "$plan_dir/ui-story-runs/US-01.md"; then
        echo 'add-ui-story emitted a cache with template placeholders.' >&2
        exit 1
    fi
fi
printf 'Planning command regression test passed.\n'
