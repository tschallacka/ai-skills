#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_review_finding_ids FILE — every finding id in the review's Findings
# table, sorted and unique, one per line. Empty output for a missing file or a
# table with no findings: that is a state, not an error.
#
# Callers use it to report what a rewrite changed rather than that it happened.
# update-adversarial-review.sh rewrites the whole table from the rows it is
# given, so a caller who derives that CSV from the table writes the same rows
# back; with no delta the success line reads the same either way, which is how
# nine findings stayed unrecorded across two cycles while the gate reported
# passed on a table that did not contain them (T66).
#
# Cells come from plan_table_cell for the reason that helper exists at all: the
# duplication ratchet counts inline pipe-splitting table parsers, and a helper
# that adds one is not a helper.
plan_review_finding_ids() {
    local file="$1" line id in_findings=0
    [ -f "$file" ] || return 0
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
            '## Findings'*) in_findings=1; continue ;;
            '## '*) in_findings=0; continue ;;
            '|'*) ;;
            *) continue ;;
        esac
        [ "$in_findings" = 1 ] || continue
        id="$(plan_table_cell "$line" 2)"
        case "$id" in AR-[0-9]*) printf '%s\n' "$id" ;; esac
    done < "$file" | sort -u
}
