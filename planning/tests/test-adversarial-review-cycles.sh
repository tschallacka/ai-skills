#!/usr/bin/env bash
# MODE: DEV
# test-adversarial-review-cycles.sh — cycle numbering and the archive guard in
# update-adversarial-review.sh.
#
# The number is the highest recorded plus one, never a count: one explicit
# --cycle above the count would otherwise send every later number backwards over
# labels already in use. The archive dedupes on the row set, so an identical
# re-run does not file twice, and a table nothing has archived yet is refused
# rather than discarded when its number is taken.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

scripts="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-review-cycles-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

note_fail() { printf 'adversarial-review-cycles: %s\n' "$1" >&2; t_record "$1"; }

# The heading numbers in file order, space-separated. An absent history and one
# with no headings are both legitimate answers, so grep is allowed to exit
# non-zero inside the pipe rather than on the assignment.
cycle_headings() {
    { grep -E '^## Cycle [0-9]+$' "$1" 2>/dev/null || true; } | awk '{ printf "%s ", $3 }'
}

assert_headings() {
    local actual
    actual="$(cycle_headings "$1")"
    [ "$actual" = "$2 " ] || note_fail "$3: headings were [$actual], expected [$2 ]"
}

assert_increasing() {
    local verdict
    verdict="$(cycle_headings "$1" | awk '
        {
            for (i = 1; i <= NF; i++) {
                if ($i + 0 <= previous + 0) { print "no"; exit }
                previous = $i
            }
            print "yes"
        }
    ')"
    [ "$verdict" = yes ] || note_fail "$2: headings are not strictly increasing: [$(cycle_headings "$1")]"
}

assert_count() {
    local actual
    actual="$({ grep -c -- "$2" "$3" 2>/dev/null || true; })"
    [ "${actual:-0}" -eq "$1" ] || note_fail "$4: found ${actual:-0} occurrences of $2, expected $1"
}

seed_plan() {
    mkdir -p "$1"
    "$scripts/create-adversarial-review.sh" "$1" >/dev/null
}

# One row, so the row set identifies the cycle it belongs to.
run_update() {
    local plan="$1" id="$2"
    shift 2
    printf 'ID,Missing or over-broad item,Required plan change,Status,Work unit\nAR-%s,the reviewer%ss gap %s,change the plan,open,N/A\n' \
        "$id" "'" "$id" \
        | "$scripts/update-adversarial-review.sh" "$plan" "$@" >/dev/null 2>&1
}

# --- a plain sequence of automatic runs numbers 1, 2, 3 and archives each ---
plan_auto="$temporary_root/plan-auto"
seed_plan "$plan_auto"
rc=0
run_update "$plan_auto" 91 || rc=$?
run_update "$plan_auto" 92 || rc=$?
run_update "$plan_auto" 93 || rc=$?
[ "$rc" -eq 0 ] || note_fail "an automatic run failed (rc=$rc)"
history_auto="$plan_auto/adversarial-review-history.md"
assert_headings "$history_auto" '1 2 3' 'three automatic runs'
assert_count 1 '| AR-91 |' "$history_auto" 'automatic runs'
assert_count 1 '| AR-92 |' "$history_auto" 'automatic runs'
assert_count 1 '| AR-93 |' "$history_auto" 'automatic runs archive the cycle they land, including the most recent'

# --- an explicit --cycle must not renumber the automatic runs after it ---
plan_mixed="$temporary_root/plan-mixed"
seed_plan "$plan_mixed"
rc=0
run_update "$plan_mixed" 11 || rc=$?
run_update "$plan_mixed" 12 || rc=$?
run_update "$plan_mixed" 13 --cycle 10 || rc=$?
run_update "$plan_mixed" 14 || rc=$?
run_update "$plan_mixed" 15 || rc=$?
[ "$rc" -eq 0 ] || note_fail "a run in the mixed sequence failed (rc=$rc)"
history_mixed="$plan_mixed/adversarial-review-history.md"
assert_headings "$history_mixed" '1 2 10 11 12' 'auto after --cycle 10'
assert_increasing "$history_mixed" 'mixed sequence'
assert_count 0 '## Cycle 4' "$history_mixed" 'the number after --cycle 10 fell back to a count'

# --- an identical re-run must not archive the same rows twice ---
plan_same="$temporary_root/plan-same"
seed_plan "$plan_same"
rc=0
run_update "$plan_same" 20 || rc=$?
run_update "$plan_same" 20 || rc=$?
run_update "$plan_same" 20 || rc=$?
[ "$rc" -eq 0 ] || note_fail "an identical re-run failed (rc=$rc)"
history_same="$plan_same/adversarial-review-history.md"
assert_headings "$history_same" '1' 'identical re-runs'
assert_count 1 '| AR-20 |' "$history_same" 'identical re-runs'

# --- a differing table whose number collides is refused, never discarded ---
plan_clash="$temporary_root/plan-clash"
seed_plan "$plan_clash"
rc=0
run_update "$plan_clash" 30 --cycle 5 || rc=$?
[ "$rc" -eq 0 ] || note_fail "the first --cycle 5 run failed (rc=$rc)"
cp "$plan_clash/adversarial-review.md" "$temporary_root/review-before.md"
cp "$plan_clash/adversarial-review-history.md" "$temporary_root/history-before.md"
rc=0
printf 'ID,Missing or over-broad item,Required plan change,Status,Work unit\nAR-31,a different gap,change the plan,open,N/A\n' \
    | "$scripts/update-adversarial-review.sh" "$plan_clash" --cycle 5 \
    >"$temporary_root/clash.out" 2>"$temporary_root/clash.err" || rc=$?
[ "$rc" -eq 73 ] || note_fail "a colliding --cycle with different rows exited $rc, expected 73"
clash_message="$(cat "$temporary_root/clash.err")"
case "$clash_message" in
    *'Cycle 5'*) : ;;
    *) note_fail "the refusal does not name the colliding cycle: $clash_message" ;;
esac
cmp -s "$temporary_root/review-before.md" "$plan_clash/adversarial-review.md" \
    || note_fail 'a refused run overwrote the previous findings table'
cmp -s "$temporary_root/history-before.md" "$plan_clash/adversarial-review-history.md" \
    || note_fail 'a refused run still wrote to the history file'
assert_count 1 '| AR-30 |' "$plan_clash/adversarial-review.md" 'the refused run'

# archive()'s "_No row-level findings were recorded for this cycle._"
# placeholder (used when the landed rows are empty) has no black-box test
# here: it archives the freshly-rendered table for THIS cycle (B353), and
# both the CSV shape gate and render() refuse an empty submission before
# archive() is ever reached, so the CLI cannot produce an empty landed table
# to exercise it with.

# T69: the Review-scope block's four self-reported fields (Reviewer
# session/Elapsed/Cost signal/Tokens) are archived alongside the Findings
# table, but Request/Repository-context-inspected are not.
set_scope_field() { # <review-file> <label> <value>
    local file="$1" label="$2" value="$3"
    awk -v label="- $label:" -v value="$value" '
        index($0, label) == 1 { print label " " value; next }
        { print }
    ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
}

# One run lands AR-60 alongside the scope block that describes it (B353: a
# reviewer fills in Reviewer session/Elapsed/Cost signal/Tokens to describe
# their OWN findings before submitting them, so the archived cycle must pair
# the two by that same authorship, not by whichever table was live at call
# time).
plan_preamble="$temporary_root/plan-preamble"
seed_plan "$plan_preamble"
review_preamble="$plan_preamble/adversarial-review.md"
set_scope_field "$review_preamble" "Reviewer session" "sentinel-session-42"
set_scope_field "$review_preamble" "Elapsed" "sentinel-elapsed-1"
set_scope_field "$review_preamble" "Cost signal" "sentinel-cost-1"
set_scope_field "$review_preamble" "Tokens" "sentinel-tokens-1"
rc=0
run_update "$plan_preamble" 60 || rc=$?
[ "$rc" -eq 0 ] || note_fail "the sentinel-preamble run failed (rc=$rc)"
history_preamble="$plan_preamble/adversarial-review-history.md"
assert_headings "$history_preamble" '1' 'the sentinel-preamble run'
cycle1="$(awk '/^## Cycle 1$/{f=1;next} /^## Cycle /{f=0} f' "$history_preamble")"
case "$cycle1" in
    *'sentinel-session-42'*) : ;;
    *) note_fail "cycle 1's own section is missing the archived Reviewer session" ;;
esac
case "$cycle1" in
    *'sentinel-elapsed-1'*) : ;;
    *) note_fail "cycle 1's own section is missing the archived Elapsed" ;;
esac
case "$cycle1" in
    *'sentinel-cost-1'*) : ;;
    *) note_fail "cycle 1's own section is missing the archived Cost signal" ;;
esac
case "$cycle1" in
    *'sentinel-tokens-1'*) : ;;
    *) note_fail "cycle 1's own section is missing the archived Tokens" ;;
esac
case "$cycle1" in
    *'Repository/context inspected'*) note_fail "cycle 1's own section archived a non-tracked field" ;;
    *) : ;;
esac
preamble_pos="$(printf '%s\n' "$cycle1" | grep -n 'sentinel-session-42' | head -1 | cut -d: -f1)"
findings_pos="$(printf '%s\n' "$cycle1" | grep -n '| AR-60 |' | head -1 | cut -d: -f1)"
[ -n "$preamble_pos" ] && [ -n "$findings_pos" ] && [ "$preamble_pos" -lt "$findings_pos" ] \
    || note_fail "cycle 1's own archived preamble did not land before its Findings table"

# A second run with the SAME AR-60 findings but a corrected scope value must
# still archive a NEW cycle, not be silently skipped by a dedup check keyed
# only on the Findings rows -- the exact regression this work unit's own
# dedup fix (mirroring the Rust archive()'s own fix) guards.
set_scope_field "$review_preamble" "Elapsed" "sentinel-elapsed-2"
set_scope_field "$review_preamble" "Tokens" "sentinel-tokens-2"
rc=0
run_update "$plan_preamble" 60 || rc=$?
[ "$rc" -eq 0 ] || note_fail "the corrected-preamble re-run failed (rc=$rc)"
assert_headings "$history_preamble" '1 2' 'the corrected-preamble re-run'
cycle2="$(awk '/^## Cycle 2$/{f=1;next} /^## Cycle /{f=0} f' "$history_preamble")"
case "$cycle2" in
    *'sentinel-elapsed-2'*) : ;;
    *) note_fail "cycle 2 does not carry the corrected Elapsed value" ;;
esac
case "$cycle2" in
    *'sentinel-tokens-2'*) : ;;
    *) note_fail "cycle 2 does not carry the corrected Tokens value" ;;
esac

# --- T56: a Verdict rationale stamped with the cycle it describes reads as
# fresh while it does, and stale once a later cycle is archived past it ---
plan_rationale="$temporary_root/plan-rationale"
"$scripts/create-plan.sh" "$plan_rationale" "T56 demo" >/dev/null
"$scripts/add-goal.sh" "$plan_rationale" 01-demo "Demo goal" "Demo outcome" >/dev/null
"$scripts/create-adversarial-review.sh" "$plan_rationale" >/dev/null

# MINTED_BY simulates a reviewer identity distinct from the claimant's own
# session, the same override verify-fix-keys' own self-certification
# refusal (B23) names as the sanctioned way around it in a single-session
# test -- never a weakening of that gate, only a stand-in for the
# would-be-separate reviewer session it exists to require.
printf 'ID,Missing or over-broad item,Required plan change,Status,Work unit\nAR-01,Missing X,Add X,resolved,W01\n' \
    | MINTED_BY=reviewer-1 "$scripts/update-adversarial-review.sh" "$plan_rationale" >/dev/null
"$scripts/update-adversarial-review.sh" "$plan_rationale" --set-rationale "AR-01 resolved; nothing outstanding." >/dev/null
review_rationale="$plan_rationale/adversarial-review.md"
case "$(cat "$review_rationale")" in
    *'- Rationale cycle: 2'*) : ;;
    *) note_fail "first --set-rationale did not stamp cycle 2 (Cycle 1 already archived, so the CURRENT table is cycle 2)" ;;
esac

key1="$(rjq -r '.keys["AR-01"]["W01"]' "$plan_rationale/fix-keys.json")"
"$scripts/add-fix-claim.sh" "$plan_rationale" --finding AR-01 --work-unit W01 --key "$key1" >/dev/null
"$scripts/update-plan-content.sh" --review-status "$plan_rationale" approved >/dev/null \
    || note_fail "approving with a fresh rationale was refused"

fresh_report="$("$scripts/validate-plan.sh" "$plan_rationale" 2>&1 || true)"
case "$fresh_report" in
    *'rationale is stale'*) note_fail "a FRESH rationale (stamped 2, history still at cycle 2) was flagged stale" ;;
    *) : ;;
esac

# Land a second cycle without touching the rationale again -- the stamp
# stays at 2, but the archive (and cycle_number computed fresh) moves to 3.
printf 'ID,Missing or over-broad item,Required plan change,Status,Work unit\nAR-02,Missing Y,Add Y,resolved,W02\n' \
    | MINTED_BY=reviewer-1 "$scripts/update-adversarial-review.sh" "$plan_rationale" >/dev/null
key2="$(rjq -r '.keys["AR-02"]["W02"]' "$plan_rationale/fix-keys.json")"
"$scripts/add-fix-claim.sh" "$plan_rationale" --finding AR-02 --work-unit W02 --key "$key2" >/dev/null
"$scripts/update-plan-content.sh" --review-status "$plan_rationale" approved >/dev/null \
    || note_fail "re-approving after a second cycle was refused"

case "$(cat "$review_rationale")" in
    *'- Rationale cycle: 2'*) : ;;
    *) note_fail "the rationale's own stamp should be untouched by a findings-only update (still cycle 2)" ;;
esac
stale_report="$("$scripts/validate-plan.sh" "$plan_rationale" 2>&1 || true)"
case "$stale_report" in
    *'rationale is stale'*) : ;;
    *) note_fail "a rationale stamped 2 with history now at cycle 3 was NOT flagged stale: $stale_report" ;;
esac

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'test-adversarial-review-cycles.sh passed.\n'
