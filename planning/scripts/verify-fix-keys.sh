#!/usr/bin/env bash
# MODE: PROD
# verify-fix-keys.sh — check every claim in fixes.md against the derived fix
# keys in fix-keys.json. Run by the approval gate before a gated review is
# marked approved; also runnable standalone.
#
# Usage:
#   The plan directory may be given positionally or as --plan-dir <path>.
#   verify-fix-keys.sh [--plan-dir] <plan-directory> [--claimed-by <id>]
#   verify-fix-keys.sh --help
#
# A plan without fix-keys.json is ungated and passes without verification.
# A gated plan must hold, per gated (finding, work-unit) pair, a fixes.md claim
# line with exactly three tab-separated fields (finding id, work unit, key) and
# the key must match HMAC-SHA256(secret, "<session_id>|<finding>|<work unit>").
# Mismatched or unclaimed pairs fail; well-formed claims for pairs that are not
# gated are ignored with a warning.
#
# Optional --claimed-by <id> names the session that recorded the claims; when it
# equals the session recorded as minted_by in fix-keys.json, the run FAILS: a
# fixer that minted and then claimed its own keys is self-certifying, which the
# gate refuses rather than reports. The approval gate always passes the flag.
# A harness whose roles share one derived session id (subagent reviewers under
# a coordinator) mints with MINTED_BY=<reviewer identity>, so the recorded
# minter is the role and honest claims never collide with it.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
# Accept --plan-dir as a synonym for the positional plan directory (the
# bounded reader takes the flag, so a reader who learned it there is not
# refused here).
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

# The `-gt 0` guard is required: expanding a possibly-empty array is unbound
# under `set -u` before bash 4.4.
tmp_files=()
cleanup_tmp() {
    if [ "${#tmp_files[@]}" -gt 0 ]; then
        rm -f "${tmp_files[@]}"
    fi
}
trap cleanup_tmp EXIT

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> [--claimed-by <id>]
       ${0##*/} --help
USAGE
    exit "$rc"
}

# A flag loop, not a `filtered_args` pre-scan: the pre-scan handled -h before
# usage() was even defined, and `set -- "${filtered_args[@]}"` is unbound under
# `set -u` before bash 4.4 when no positional argument was given.
claimed_by=""
plan_dir=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --claimed-by) [ "$#" -ge 2 ] || usage; claimed_by="$2"; shift 2 ;;
        --claimed-by=*) claimed_by="${1#--claimed-by=}"; shift ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *)
            [ -z "$plan_dir" ] || usage
            plan_dir="$1"
            shift
            ;;
    esac
done
[ -n "$plan_dir" ] || usage

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
    local fixes_file="$1/fixes.md" session_id minted_by secret_file secret
    local pairs_file claims_file line_no failures warnings n fid wu key expected line
    local claimed_by="${2:-}"
    [ -f "$review_file" ] || plan_die "adversarial-review.md not found: $review_file"

    if [ ! -f "$json_file" ]; then
        printf 'ungated plan (no fix-keys.json): no fix verification required\n'
        return 0
    fi

    session_id="$(sed -n 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
        "$json_file" | head -1)"
    [ -n "$session_id" ] || plan_die "fix-keys.json has no session_id"
    minted_by="$(sed -n 's/.*"minted_by"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
        "$json_file" | head -1)"

    # Named templates so a leaked temp is identifiable as this script's.
    pairs_file="$(mktemp "${TMPDIR:-/tmp}/verify-fix-keys-pairs.XXXXXX")"
    claims_file="$(mktemp "${TMPDIR:-/tmp}/verify-fix-keys-claims.XXXXXX")"
    tmp_files+=("$pairs_file" "$claims_file")

    plan_review_gated_pairs "$review_file" > "$pairs_file"

    if [ ! -s "$pairs_file" ]; then
        printf 'no gated (finding, work unit) pairs in %s: no fix verification required\n' "$plan_dir"
        return 0
    fi

    # Refuse with 69 rather than report a mismatch that is really a missing
    # tool. The check sits after the ungated early-returns so an ungated plan
    # still passes without openssl installed.
    command -v openssl >/dev/null 2>&1 || \
        plan_die 'openssl (or LibreSSL) is required to verify fix keys via HMAC-SHA256' 69

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

    # Counted with the key failures and checked before them, so a self-certified
    # claim set cannot pass on a caller that only reads the exit status.
    if [ -n "$claimed_by" ] && [ -n "$minted_by" ] && [ "$claimed_by" = "$minted_by" ]; then
        printf 'self-certification: fix claims for %s were recorded by the same session (%s) that minted the keys -- let the reviewer write adversarial-review-incoming.md so the findings arrive from its own session, have a fresh reviewer re-review and re-mint, or, in a harness whose roles share one derived session id, re-mint with MINTED_BY=<reviewer identity> so the recorded minter is the role\n' \
            "$plan_dir" "$claimed_by" >&2
        failures=$((failures + 1))
    fi

    if [ "$failures" -gt 0 ]; then
        plan_die "$(printf 'fix-keys verification failed for %s (%s failure(s), %s warning(s))' "$plan_dir" "$failures" "$warnings")"
    fi
    printf 'fix-keys verification passed for %s (%s warning(s))\n' "$plan_dir" "$warnings"
}

main() {
    plan_require_directory "$plan_dir"
    verify_fix_keys "$plan_dir" "$claimed_by"
}

main
