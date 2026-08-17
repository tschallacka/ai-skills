#!/usr/bin/env bash
# verify-fix-keys.sh — check every claim in fixes.md against the derived fix
# keys in fix-keys.json. Run by the approval gate before a gated review is
# marked approved; also runnable standalone.
#
# A plan without fix-keys.json is ungated and passes without verification.
# A gated plan must hold, per gated (finding, work-unit) pair, a fixes.md claim
# line with exactly three tab-separated fields (finding id, work unit, key) and
# the key must match HMAC-SHA256(secret, "<session_id>|<finding>|<work unit>").
# Mismatched or unclaimed pairs fail; well-formed claims for pairs that are not
# gated are ignored with a warning.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/plan-document-lib.sh"

tmp_files=()
cleanup_tmp() {
    if [ "${#tmp_files[@]}" -gt 0 ]; then
        rm -f "${tmp_files[@]}"
    fi
}
trap cleanup_tmp EXIT

usage() {
    printf 'Usage: %s <plan-directory>\n' "$(basename "$0")" >&2
    exit 64
}

# hmac_key SECRET MESSAGE — lowercase hex HMAC-SHA256 of MESSAGE under SECRET.
# Must stay byte-identical to the derivation in mint-fix-keys.sh.
hmac_key() {
    local secret="$1" message="$2"
    printf '%s' "$message" | openssl dgst -sha256 -hmac "$secret" -binary \
        | od -An -vtx1 | tr -d ' \n'
}

verify_fix_keys() {
    local plan_dir="$1"
    local review_file="$1/adversarial-review.md" json_file="$1/fix-keys.json"
    local fixes_file="$1/fixes.md" session_id secret_file secret
    local pairs_file claims_file line_no failures warnings n fid wu key
    [ -f "$review_file" ] || plan_die "adversarial-review.md not found: $review_file"

    if [ ! -f "$json_file" ]; then
        printf 'ungated plan (no fix-keys.json): no fix verification required\n'
        return 0
    fi

    session_id="$(sed -n 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
        "$json_file" | head -1)"
    [ -n "$session_id" ] || plan_die "fix-keys.json has no session_id"

    pairs_file="$(mktemp)"
    claims_file="$(mktemp)"
    tmp_files+=("$pairs_file" "$claims_file")

    awk -F'|' '
        /^## Findings$/ { in_findings = 1; next }
        in_findings && /^## Verdict$/ { exit }
        in_findings && /^\|/ {
            fid = $2; wu = $6
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", fid)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", wu)
            if (fid ~ /^AR-[0-9]+$/ && wu ~ /^W[0-9]+$/) print fid "\t" wu
        }
    ' "$review_file" > "$pairs_file"

    if [ ! -s "$pairs_file" ]; then
        printf 'no gated (finding, work unit) pairs in %s: no fix verification required\n' "$plan_dir"
        return 0
    fi

    secret_file="$(printf '%s/review-fix-keys/%s/secret\n' "$(planning_tmpdir)" "$session_id")"
    [ -f "$secret_file" ] || plan_die "session secret missing: $secret_file (was the session invalidated at approval?)"
    [ -f "$fixes_file" ] || plan_die "fixes.md missing; a gated plan must record one claim per (finding, work unit)"
    secret="$(cat "$secret_file")"

    failures=0
    warnings=0
    line_no=0
    while IFS= read -r line; do
        line_no=$((line_no + 1))
        [ -n "$line" ] || continue
        if ! awk -F'\t' 'NF == 3 { exit 0 } { exit 1 }' <<< "$line"; then
            printf 'malformed fixes.md claim (line %s): expected finding_id, work_unit, key\n' "$line_no" >&2
            failures=$((failures + 1))
            continue
        fi
        fid="$(printf '%s' "$line" | awk -F'\t' '{print $1}')"
        wu="$(printf '%s' "$line" | awk -F'\t' '{print $2}')"
        key="$(printf '%s' "$line" | awk -F'\t' '{print $3}')"
        if ! grep -Fqx "$fid	$wu" "$pairs_file"; then
            printf 'ignoring claim for pair %s/%s (not gated in fix-keys.json)\n' "$fid" "$wu" >&2
            warnings=$((warnings + 1))
            continue
        fi
        expected="$(hmac_key "$secret" "$session_id|$fid|$wu")"
        if [ "$expected" != "$key" ]; then
            printf 'fix key mismatch for %s/%s (forged or stale key)\n' "$fid" "$wu" >&2
            failures=$((failures + 1))
        fi
        printf '%s\t%s\n' "$fid" "$wu" >> "$claims_file"
    done < "$fixes_file"

    while IFS=$'\t' read -r fid wu; do
        [ -n "$fid" ] || continue
        if ! grep -Fqx "$fid	$wu" "$claims_file"; then
            printf 'no fix key claim recorded for gated pair %s/%s\n' "$fid" "$wu" >&2
            failures=$((failures + 1))
        fi
    done < "$pairs_file"

    if [ "$failures" -gt 0 ]; then
        plan_die "$(printf 'fix-keys verification failed for %s (%s failure(s), %s warning(s))' "$plan_dir" "$failures" "$warnings")"
    fi
    printf 'fix-keys verification passed for %s (%s warning(s))\n' "$plan_dir" "$warnings"
}

main() {
    [ "$#" -eq 1 ] || usage
    plan_require_directory "$1"
    verify_fix_keys "$1"
}

main "$@"
