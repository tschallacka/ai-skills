#!/usr/bin/env bash
# MODE: DEV
# test-coherence-checks — B110: validate-plan.sh's intra-document
# self-coherence sweep (T138 stale-wording-retained, T139
# countable-enumeration). Six reviewer cycles on a real plan were each gated
# by an intra-document inconsistency a mechanical sweep could have caught
# before a reviewer was ever dispatched (see BUGS.json B110). These checks are
# FAIL, not WARN like the --stale phrase sweep, because each is an exact
# substring/pattern match rather than a phrase-list heuristic.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# script_dir locates the sibling .awk files the lib's scan functions invoke;
# validate-plan.sh sets this global before sourcing the lib in the real
# pipeline, so this file does the same to exercise it in isolation.
script_dir="$repo_root/planning/scripts"
# shellcheck source=planning/scripts/validate-plan-coherence-lib.sh
source "$repo_root/planning/scripts/validate-plan-coherence-lib.sh"

note_fail() { printf 'coherence-checks: %s\n' "$1" >&2; t_record "$1"; }

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/coherence-checks.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

write_doc() { # <name> -- body on stdin, path left in $doc
    doc="$temporary_root/$1.md"
    cat > "$doc"
}

# The scan functions read $stale_markers as a global (published by
# validate-plan-stale-lib.sh in the real pipeline); set it directly here so
# this file can test the coherence lib in isolation.
stale_markers='an earlier version|previously|superseded by|supersedes|no longer|was removed|historically|now replaced by'

# ═════════════════════════════════════════════════════════════════════════
# T138 — stale wording retained
# ═════════════════════════════════════════════════════════════════════════

# ── the incident's own shape: a retracted quoted claim survives elsewhere ───
write_doc self_contradiction <<'MD'
## Objective

leave dropship buckets unchanged

## Instructions

An earlier version of this said "leave dropship buckets unchanged"; that no
longer applies since the migration.
MD
hits="$(plan_coherence_scan_stale_wording "$doc")"
case "$hits" in
    '2'$'\t''1'$'\t'*'leave dropship buckets unchanged'*) ;;
    *) note_fail "the exact incident shape (double-quoted claim) was not flagged: $hits" ;;
esac

# ── single-quoted claim, same shape ──────────────────────────────────────────
write_doc single_quoted <<'MD'
## Row

autoplay exists in every mode

## Correction

An earlier version of this row said 'autoplay exists in every mode', which
finding AR-08 recorded as wrong.
MD
hits="$(plan_coherence_scan_stale_wording "$doc")"
case "$hits" in
    *'autoplay exists in every mode'*) ;;
    *) note_fail "a single-quoted retracted claim was not flagged: $hits" ;;
esac

# ── a correctly-updated document, no survival, must pass ────────────────────
write_doc correctly_updated <<'MD'
## Objective

Buckets ship exactly as configured today.

## Instructions

An earlier version of this said "leave dropship buckets unchanged"; that no
longer applies since the migration.
MD
hits="$(plan_coherence_scan_stale_wording "$doc")"
[ -z "$hits" ] || note_fail "a correctly-updated document with no survival was flagged: $hits"

# ── an unrelated use of the retracted text elsewhere must not confuse ────────
# The retracted claim only overlaps by coincidence if a document never
# actually repeats it; this fixture instead proves the check does not need
# any such coincidence to stay quiet when the claim genuinely is not repeated.
write_doc unrelated_text <<'MD'
## Objective

Ship exactly what the request specifies, nothing else.

## Instructions

An earlier version of this said "leave dropship buckets unchanged"; that no
longer applies since the migration.

## Elsewhere

The wording "totally different content" appears once, only here.
MD
hits="$(plan_coherence_scan_stale_wording "$doc")"
[ -z "$hits" ] || note_fail "unrelated quoted text produced a false positive: $hits"

# ── the contraction guard: an apostrophe inside a word is not a quote ────────
write_doc contraction_guard <<'MD'
## Marked paragraph with an apostrophe but nothing quoted

An earlier version of the unit's own instructions was wrong, but nothing here
is quoted at all, so the apostrophe in "unit's" must not be parsed as an
opening quote.
MD
hits="$(plan_coherence_scan_stale_wording "$doc")"
[ -z "$hits" ] || note_fail "a contraction's apostrophe was misparsed as a quote: $hits"

# ── a second retraction paragraph repeating the same claim is exempt ─────────
# Restating what changed a second time is not the half-landed-fix shape this
# check exists to find.
write_doc second_retraction <<'MD'
## Objective

leave dropship buckets unchanged

## First correction

An earlier version of this said "leave dropship buckets unchanged"; that no
longer applies.

## Second correction, same claim repeated

An earlier version of this row also said 'leave dropship buckets unchanged',
which finding AR-08 recorded as wrong and superseded here.
MD
hits="$(plan_coherence_scan_stale_wording "$doc")"
survivor_count="$(printf '%s\n' "$hits" | grep -c $'\t''3'$'\t' || true)"
t_assert_eq 'a second retraction paragraph is not reported as a survivor' "$survivor_count" '0'
case "$hits" in
    *$'\t'1$'\t'*) ;;
    *) note_fail "the objective paragraph (1) was not reported as a survivor: $hits" ;;
esac

# ── end to end through validate-plan.sh, not just the scan function ─────────
scripts_dir="$repo_root/planning/scripts"
e2e_plan="$temporary_root/e2e-plan"
cp -R "$repo_root/benchmark/planning/fixtures/plans/untrack-generated-files" "$e2e_plan" 2>/dev/null \
    || cp -R "$repo_root/planning/tests/fixtures/overview/navigation" "$e2e_plan"
e2e_doc="$(find "$e2e_plan" -maxdepth 1 -name 'plan-description.md')"
if [ -n "$e2e_doc" ] && [ -f "$e2e_doc" ]; then
    e2e_pristine="$temporary_root/e2e.pristine"
    cp "$e2e_doc" "$e2e_pristine"
    printf '\n## Retraction probe\n\nleave dropship buckets unchanged\n\n## Retraction probe correction\n\nAn earlier version of this said "leave dropship buckets unchanged"; that no longer applies.\n' \
        >> "$e2e_doc"
    out="$("$scripts_dir/validate-plan.sh" "$e2e_plan" 2>&1 || true)"
    case "$out" in
        *'still carries it verbatim'*) ;;
        *) note_fail "validate-plan.sh did not surface the coherence FAIL end to end: $(printf '%s' "$out" | grep -c '^FAIL' || true) FAILs total" ;;
    esac
    cp "$e2e_pristine" "$e2e_doc"
fi

# ═════════════════════════════════════════════════════════════════════════
# T139 — countable set with no explicit enumeration
# ═════════════════════════════════════════════════════════════════════════

# ── the incident's own shape: a count with nothing named ────────────────────
write_doc no_members <<'MD'
## Handoff

The following four steps each add one public method to the interface.
MD
hits="$(plan_coherence_scan_countable_enumeration "$doc")"
case "$hits" in
    *'The following four steps'*) ;;
    *) note_fail "an unenumerated count was not flagged: $hits" ;;
esac

# ── enumerated by WNN ids in the same paragraph: must pass ───────────────────
write_doc members_wnn <<'MD'
## Handoff

The following three steps -- W12, W14 and W19 -- each add one public method.
MD
hits="$(plan_coherence_scan_countable_enumeration "$doc")"
[ -z "$hits" ] || note_fail "a WNN-enumerated count was flagged: $hits"

# ── enumerated by file paths in the same paragraph: must pass ───────────────
write_doc members_paths <<'MD'
## Handoff

The following two files -- planning/scripts/foo.sh and planning/scripts/bar.sh -- change.
MD
hits="$(plan_coherence_scan_countable_enumeration "$doc")"
[ -z "$hits" ] || note_fail "a path-enumerated count was flagged: $hits"

# ── enumerated by BNN/TNN ids: must pass ─────────────────────────────────────
write_doc members_bugs <<'MD'
## Handoff

The following two bugs -- B91 and B93 -- block this step.
MD
hits="$(plan_coherence_scan_countable_enumeration "$doc")"
[ -z "$hits" ] || note_fail "a BNN-enumerated count was flagged: $hits"

# ── the excluded broader forms: a universal count and anaphora must pass ────
# T139's own scope note: measured against the real corpus, "all N X" and
# "these/those X" were 0% precise (universal counts and back-references, not
# unenumerated promises), so only "the following N X" is gated.
write_doc excluded_universal <<'MD'
## Dashboard

All twelve goals are represented in the dashboard.
MD
hits="$(plan_coherence_scan_countable_enumeration "$doc")"
[ -z "$hits" ] || note_fail "a universal count (all N of the total) was flagged: $hits"

write_doc excluded_anaphora <<'MD'
## Fix

Those files were already listed above and need no repeating here.
MD
hits="$(plan_coherence_scan_countable_enumeration "$doc")"
[ -z "$hits" ] || note_fail "anaphoric back-reference prose was flagged: $hits"

# ── case-insensitivity: a capitalised "The following" still triggers ────────
write_doc capitalised <<'MD'
## Handoff

The following Four steps each add one public method to the interface.
MD
hits="$(plan_coherence_scan_countable_enumeration "$doc")"
[ -n "$hits" ] || note_fail "a capitalised trigger phrase was not matched"

echo
echo "coherence-checks: $([ "$(t_failures)" -eq 0 ] && echo PASS || echo FAIL)"
[ "$(t_failures)" -eq 0 ]
