#!/usr/bin/env bash
# MODE: PROD
# validate-plan-stale-lib.sh — the --stale sweep: a listed phrase surviving in a
# paragraph that records no history marker may be a half-landed fix.
#
# Advisory throughout: every finding is a WARN. Measured against a labelled
# corpus the count phrases found 0 defects in 24 real hits -- every one was an
# accurate count of a fixed set, usually enumerated in the same sentence -- and
# the identical-output phrases were 50% precise. Neither can separate a defect
# from correct writing, because the deciding fact (did the list grow? is the
# artifact deterministic?) is not in the phrase. A gate that reports style as
# defect gets dismissed wholesale, which is what happened to all 13 findings on
# one real plan. So this reports, and the checks that can be exact do the gating:
# validate-plan-comparisons-lib.sh for declared comparisons, and the
# paragraph-level marker rule below for a genuinely half-landed fix.
#
# Sourced by validate-plan.sh; never executed. Requires
# validate-plan-common-lib.sh and the `plan_docs` list from
# validate-plan-docs-lib.sh. Registers its generated phrase file with the entry
# script's `cleanup_files` list rather than installing its own EXIT trap — a
# `trap - EXIT` here would discard the process-wide handler.

# shellcheck disable=SC2154
# The entry script and its sibling libs publish these globals; shellcheck lints
# each file alone and cannot see the assignments.
set -euo pipefail

# Default stale phrase list: a case count drifts the moment a case is added, so
# these flag the countable form for conversion to an explicit enumeration. A
# paragraph that also records a history marker is exempt.
stale_default_phrases=(
    'all four'
    'all six'
    'all three'
    'all five'
    'the eleven'
    'the six states'
    'all states'
    'four per-state'
)

# Wording that asks for an identical output. Measured against a labelled corpus
# this is 50% precise as a gate -- an identical PDF is a real defect, while
# `byte-for-byte in oracle-terminal-evidence.json` is correct -- because the
# discriminating fact is the artifact and prose does not carry it. So it WARNS
# and points at the table that does: validate-plan-comparisons-lib.sh checks a
# declared comparison exactly.
stale_comparison_phrases=(
    'byte-identical'
    'byte-for-byte'
    'pixel-identical'
    'exactly identical'
)
# --- stale-wording sweep (--stale <file-of-phrases>): a listed phrase in a
#     paragraph that records no history marker is stale. Markers are multi-word
#     ONLY: "legacy" alone may be part of the phrase and would mask the match.
stale_markers='an earlier version|previously|superseded by|supersedes|no longer|was removed|historically|now replaced by'

# Buffering is per PARAGRAPH, never per section: a marker in one paragraph must
# not exempt an unfixed sibling under the same heading, which is the half-landed
# fix this sweep exists to find. The report names the paragraph, not the section.
stale_scan_doc() {
    local file="$1" phrase="$2"
    awk -v phrase="$phrase" -v markers="$stale_markers" '
        function flush() {
            if (label != "" && flat != "") {
                paragraph++
                if (index(content, phrase) > 0 && tolower(flat) !~ markers) {
                    printf "%s: %s [paragraph %d] %s\n", FILENAME, label, paragraph,
                        (length(flat) > 120 ? substr(flat, 1, 117) "..." : flat)
                }
            }
            content = ""; flat = ""
        }
        /^#+ / { flush(); label = $0; paragraph = 0; next }
        /^[[:space:]]*$/ { flush(); next }
        {
            if (label != "") {
                content = content $0 "\n"
                flat = (flat == "" ? $0 : flat " " $0)
            }
        }
        END { flush() }
    ' "$file"
}

plan_validate_stale() {
    # The stale sweep INCLUDES the *-testing.md companions, the surface most
    # likely to drift. plan_docs excludes companions for the structural
    # hardening checks, so build a separate list here.
    stale_docs=("${plan_docs[@]}")
    for step_file in "$plan_dir"/*/steps/*-testing.md; do
        [ -f "$step_file" ] || continue
        stale_docs+=("$step_file")
    done
    if [ "$stale_requested" = true ]; then
        # --stale default (or --stale with no file) uses the bundled case-count
        # phrase list; --stale <file> uses that file. When a file is given, it is
        # used alone (extend it by adding phrases to the file).
        if [ -n "$stale_file" ] && [ "$stale_file" != "default" ]; then
            if [ ! -f "$stale_file" ]; then
                fail "--stale file not found: $stale_file"
            fi
            stale_phrases_file="$stale_file"
            # A caller-supplied list replaces the bundled one, so the bundled
            # comparison phrases must not also run alongside it.
            stale_comparison_active=false
        else
            stale_comparison_active=true
            stale_phrases_file="$(mktemp "${TMPDIR:-/tmp}/plan-stale-default.XXXXXX")"
            # Register with the entry script's single accumulating cleanup
            # rather than an EXIT trap here: `trap - EXIT` to "release" it
            # would discard the process-wide handler too (CODE-STYLE.md §8).
            cleanup_files+=("$stale_phrases_file")
            printf '%s\n' "${stale_default_phrases[@]}" > "$stale_phrases_file"
        fi
        while IFS= read -r phrase; do
            [ -n "${phrase//[[:space:]]/}" ] || continue
            for doc in "${stale_docs[@]}"; do
                [ -f "$doc" ] || continue
                hits="$(stale_scan_doc "$doc" "$phrase")"
                if [ -n "$hits" ]; then
                    warn "count '$phrase' in an unmarked paragraph: $(printf '%s' "$hits" | tr '\n' ' ') -- a count drifts the moment a case is added, so enumerate the items or name the section that lists them"
                fi
            done
        done < "$stale_phrases_file"
        [ "$stale_comparison_active" = true ] || return 0
        local comparison
        for comparison in ${stale_comparison_phrases[@]+"${stale_comparison_phrases[@]}"}; do
            for doc in "${stale_docs[@]}"; do
                [ -f "$doc" ] || continue
                hits="$(stale_scan_doc "$doc" "$comparison")"
                [ -n "$hits" ] || continue
                warn "wording '$comparison' in an unmarked paragraph: $(printf '%s' "$hits" | tr '\n' ' ') -- if this is an acceptance criterion, declare it in the step's '## Artifact comparisons' table (update-plan-content.sh -tp) so the comparison is checked instead of guessed"
            done
        done
        if [ -z "$stale_file" ] || [ "$stale_file" = "default" ]; then
            rm -f "$stale_phrases_file"
        fi
    fi
}
