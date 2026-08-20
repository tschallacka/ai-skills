#!/usr/bin/env bash
# The (finding, work unit) pairs the fix-key gate covers, as `AR-NN<TAB>WNN`.
# One parser for the review's five-column Findings table, because three scripts
# had their own copy of it: mint-fix-keys.sh derives a key per pair,
# verify-fix-keys.sh checks a claim against those pairs, and add-fix-claim.sh
# refuses a claim for a pair nothing gates. Three copies of the field indices is
# three chances for the writer to accept what the verifier does not gate.
#
# Non-conforming ids are excluded here, as they were in every copy; mint reports
# them separately before calling this, because a row it cannot mint disables the
# gate silently.
plan_review_gated_pairs() {
    local review_file="$1"
    [ -f "$review_file" ] || return 0
    awk -F'|' '
        /^## Findings$/ { in_findings = 1; next }
        in_findings && /^## Verdict$/ { exit }
        in_findings && /^\|/ {
            fid = $2; wu = $6
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", fid)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", wu)
            if (fid ~ /^AR-[0-9]+$/ && wu ~ /^W[0-9]+$/) print fid "\t" wu
        }
    ' "$review_file"
}
