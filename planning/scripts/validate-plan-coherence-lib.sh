#!/usr/bin/env bash
# MODE: PROD
# validate-plan-coherence-lib.sh — B110: a coordinator's own mechanical
# self-coherence sweep, run before a reviewer is dispatched. Six reviewer
# cycles on a real plan were each gated by a purely intra-document
# inconsistency a script could have caught, in a surface the previous fix had
# not touched (see BUGS.json B110). The --stale sweep is the closest existing
# mechanism and does not close the loop: every hit there is a WARN a caller
# supplies from memory, never a FAIL. These checks are FAIL, because they are
# exact rather than a phrase-list heuristic -- see each function's own header
# for what makes it exact.
#
# Sourced by validate-plan.sh, AFTER validate-plan-stale-lib.sh: this file
# reads $stale_docs and $stale_markers, both published by that library's
# plan_validate_stale(), rather than rebuilding the companion-inclusive
# document list and the retraction-marker vocabulary a second time.
#
# Never executed.

# shellcheck disable=SC2154
# stale_docs and stale_markers are published by validate-plan-stale-lib.sh;
# script_dir (locating the sibling .awk files below) and errors/fail()/warn()
# by validate-plan.sh and validate-plan-common-lib.sh. shellcheck lints each
# file alone and cannot see those assignments.
set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# T138 -- a document that still carries wording it declared stale.
#
# The incident: a work unit's Objective read "leave dropship buckets
# unchanged" while its own Instructions called that exact wording "an earlier
# version". Both paragraphs are individually fine; together they assert the
# document was updated AND that it says what it said before, about the same
# quoted claim. Mechanical and precise, not a heuristic, because the trigger is
# an exact substring match on text the paragraph ITSELF quotes as retracted --
# nothing is inferred about which sentence means what.
#
# A "retraction paragraph" is one that both matches $stale_markers (the same
# vocabulary the --stale sweep already recognises) AND quotes a claim: a
# double-quoted span, or a single-quoted span whose opening quote is not
# itself a contraction's apostrophe (guarded by requiring the character before
# it not be a letter, and the character after its closing match not be a
# letter -- "the unit's own instructions" contains no such pair; "'leave
# dropship buckets unchanged'" does). Claims under 8 characters are ignored as
# noise (a quoted single word is rarely the retracted content).
#
# A claim survives when its exact text (case-sensitive: the whole point is
# that the SAME wording remains) appears in another paragraph of the same
# document that is not itself a retraction paragraph -- a second retraction
# paragraph quoting the same claim again is restating what changed, not
# leaving it in force.
# ─────────────────────────────────────────────────────────────────────────────
# The awk program itself lives in validate-plan-stale-wording.awk:
# CODE-STYLE.md caps an inline awk program at 15 lines before it must move to
# a lib function or a file, and character-scanned quote extraction (POSIX awk
# has no capturing match()) does not fit in 15.
plan_coherence_scan_stale_wording() { # <file>
    awk -v markers="$stale_markers" -f "$script_dir/validate-plan-stale-wording.awk" "$1"
}

plan_validate_stale_wording_retained() {
    local doc retraction_p survival_p snippet
    for doc in "${stale_docs[@]}"; do
        [ -f "$doc" ] || continue
        while IFS="$(printf '\t')" read -r retraction_p survival_p snippet; do
            [ -n "$retraction_p" ] || continue
            fail "$doc: paragraph $retraction_p declares '$snippet' stale, but paragraph $survival_p still carries it verbatim -- update or remove the stale wording"
        done < <(plan_coherence_scan_stale_wording "$doc")
    done
}

# ─────────────────────────────────────────────────────────────────────────────
# T139 -- "the following N <things>" with no explicit list of the N.
#
# The incident: a handoff claimed "the following four steps each add one
# public method" after a fifth unit had moved in -- a count that drifted the
# moment the set changed, with nothing next to it a validator (or a reader)
# could check it against.
#
# The bug's original idea was broader: flag ANY spelled-out count or
# collective phrase ("all three files", "these bugs"). Measured against the
# real corpus this repository's own plan-overview-rebuild plan produced
# (planning/tests/fixtures/overview/size), that broader shape is imprecise in
# the same way the --stale sweep's own count phrases already are (0/24
# precision, per that file's header): "all twelve goals" is a correct
# universal count, and "these files"/"those stories" are ordinary anaphora
# referring back to a list already given earlier, not a same-paragraph promise
# left unkept. Every one of ~28 hits for the broader shape was a false
# positive on that corpus; zero hits (positive or false) came back for "the
# following N <noun>" specifically, which is the one form that grammatically
# PROMISES an explicit list follows immediately, so its absence is a genuine,
# checkable gap rather than a judgement call. This check is scoped to exactly
# that narrower, precise form. The broader phrasing remains covered, at WARN
# severity, by the existing --stale count-phrase sweep.
#
# A member list is "explicit" when the SAME paragraph names at least one
# concrete referent: a WNN work-unit id, a BNN/TNN bug or todo id, or a
# file-path-like token (contains a slash, or a dot followed by letters, such
# as a bare relative path or an extension). Absent all three, the count has
# nothing in the same paragraph a reader -- or this check -- could verify it
# against.
# ─────────────────────────────────────────────────────────────────────────────
# The awk program itself lives in validate-plan-countable-enumeration.awk (see
# the note on plan_coherence_scan_stale_wording above for why).
plan_coherence_scan_countable_enumeration() { # <file>
    awk -f "$script_dir/validate-plan-countable-enumeration.awk" "$1"
}

plan_validate_countable_enumeration() {
    local doc paragraph_n snippet
    for doc in "${stale_docs[@]}"; do
        [ -f "$doc" ] || continue
        while IFS="$(printf '\t')" read -r paragraph_n snippet; do
            [ -n "$paragraph_n" ] || continue
            fail "$doc: paragraph $paragraph_n [$snippet] names a count with no explicit member in the same paragraph -- list the specific ids/paths, or reduce to a plain count with no promise of a following list"
        done < <(plan_coherence_scan_countable_enumeration "$doc")
    done
}
