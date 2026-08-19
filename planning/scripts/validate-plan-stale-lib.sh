#!/usr/bin/env bash
# validate-plan-stale-lib.sh — the --stale sweep: a listed phrase that survives
# in a paragraph carrying no history marker is a half-landed fix.
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
    # Class B (report 12): a criterion demanding an identical/byte-identical
    # output the target cannot produce (embedded metadata, live dates) is a
    # gate that fails correct work. Flag the wording so it is revisited.
    'byte-identical'
    'byte-for-byte'
    'pixel-identical'
    'exactly identical'
)
# --- stale-wording sweep (--stale <file-of-phrases>): a listed phrase in a
#     paragraph that records no history marker is stale. Markers are multi-word
#     ONLY: "legacy" alone may be part of the phrase and would mask the match.
stale_markers='an earlier version|previously|superseded by|supersedes|no longer|was removed|historically|now replaced by'

# The awk prelude behind the sweep: the marker test is case-insensitive because
# SKILL.md's documented correction note starts a sentence, and a count already
# discharged by an enumeration in the same paragraph is the target form, not a hit.
stale_awk_predicates() {
    cat <<'AWK'
function stale_count(p) {
    if (index(p, "eleven")) { return 11 }
    if (index(p, "twelve")) { return 12 }
    if (index(p, "three")) { return 3 }
    if (index(p, "seven")) { return 7 }
    if (index(p, "eight")) { return 8 }
    if (index(p, "four")) { return 4 }
    if (index(p, "five")) { return 5 }
    if (index(p, "nine")) { return 9 }
    if (index(p, "six")) { return 6 }
    if (index(p, "ten")) { return 10 }
    if (index(p, "two")) { return 2 }
    if (match(p, /[0-9]+/)) { return substr(p, RSTART, RLENGTH) + 0 }
    return 0
}
# A bare comma run is ordinary prose; an enumeration announces itself first.
function stale_introduced(head) {
    return (index(head, ":") || index(head, "\342\200\224") ||
        index(head, "\342\200\223") || index(head, " - "))
}
function stale_enumerated(text, phrase, n,   tail, head, copy, seen, tok, ids, seps, at) {
    if (n < 2) { return 0 }
    at = index(text, phrase)
    if (at == 0) { return 0 }
    tail = substr(text, at + length(phrase))
    copy = tail
    ids = 0
    while (match(copy, /W[0-9][0-9]/)) {
        tok = substr(copy, RSTART, RLENGTH)
        if (!(tok in seen)) { seen[tok] = 1; ids++ }
        copy = substr(copy, RSTART + RLENGTH)
    }
    if (ids >= n) { return 1 }
    at = match(tail, /[,;]/)
    if (at == 0) { return 0 }
    if (!stale_introduced(substr(tail, 1, at))) { return 0 }
    copy = tail
    seps = gsub(/[,;]/, "x", copy)
    return (seps >= n - 1)
}
AWK
}

# Buffering is per PARAGRAPH, never per section: a marker in one paragraph must
# not exempt an unfixed sibling under the same heading, which is the half-landed
# fix this sweep exists to find. The report names the paragraph, not the section.
stale_scan_doc() {
    local file="$1" phrase="$2"
    awk -v phrase="$phrase" -v markers="$stale_markers" "$(stale_awk_predicates)"'
        function flush() {
            if (label != "" && flat != "") {
                paragraph++
                if (index(content, phrase) > 0 && tolower(flat) !~ markers &&
                        !stale_enumerated(flat, phrase, stale_count(phrase))) {
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
        else
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
                    fail "stale phrase '$phrase' appears in an unmarked paragraph: $(printf '%s' "$hits" | tr '\n' ' ')"
                fi
            done
        done < "$stale_phrases_file"
        if [ -z "$stale_file" ] || [ "$stale_file" = "default" ]; then
            rm -f "$stale_phrases_file"
        fi
    fi
}
