#!/usr/bin/env bash
# add-fix-claim.sh — record one fix-key claim in a plan's fixes.md.
#
# fixes.md had five readers and no writer. The fixer was expected to produce it,
# yet SKILL.md forbids hand-authoring a plan artifact, so the only way to satisfy
# the fix-key gate was to break that rule. This is the writer.
#
# Usage:
#   add-fix-claim.sh [--plan-dir] <plan-directory> --finding <AR-NN> \
#       --work-unit <WNN> --key <hex>
#   add-fix-claim.sh --help
#
# One claim per call, appended as `finding_id \t work_unit \t key`, which is the
# shape verify-fix-keys.sh reads.
#
# The key is checked for shape and for being gated, never derived: deriving it
# needs the minting session's secret, and a fixer that could reach that secret
# could mint its own keys. Cryptographic verification stays in
# verify-fix-keys.sh, run by a session that is not the minting one.

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> --finding <AR-NN> --work-unit <WNN> --key <hex>
       ${0##*/} --help

  --finding AR-NN    the adversarial-review finding the fix answers
  --work-unit WNN    the work unit that carried the fix
  --key hex          the fix key minted for that pair (64 lowercase hex chars)

Records one claim in <plan-directory>/fixes.md. Run verify-fix-keys.sh from a
session that did not mint the keys to check the claims.
USAGE
    exit "$rc"
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"

finding=""
work_unit=""
key=""
positional=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --plan-dir) [ "$#" -ge 2 ] || usage; positional+=("$2"); shift 2 ;;
        --finding) [ "$#" -ge 2 ] || usage; finding="$2"; shift 2 ;;
        --work-unit) [ "$#" -ge 2 ] || usage; work_unit="$2"; shift 2 ;;
        --key) [ "$#" -ge 2 ] || usage; key="$2"; shift 2 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) positional+=("$1"); shift ;;
    esac
done
while [ "$#" -gt 0 ]; do positional+=("$1"); shift; done
set -- ${positional[@]+"${positional[@]}"}
[ "$#" -eq 1 ] || usage
plan_dir="$1"

# ---- validate before touching the file -------------------------------------
plan_require_directory "$plan_dir"
[ -n "$finding" ] || plan_die "--finding is required (an adversarial-review id such as AR-01)"
[ -n "$work_unit" ] || plan_die "--work-unit is required (a work-unit id such as W05)"
[ -n "$key" ] || plan_die "--key is required (the fix key minted for this pair)"
[[ "$finding" =~ ^AR-[0-9]+$ ]] || plan_die "Finding id must match AR-NN: $finding"
[[ "$work_unit" =~ ^W[0-9]+$ ]] || plan_die "Work unit must match WNN: $work_unit"
[[ "$key" =~ ^[0-9a-f]{64}$ ]] || plan_die "Key must be 64 lowercase hex characters as minted by mint-fix-keys.sh: $key"

keys_file="$plan_dir/fix-keys.json"
[ -f "$keys_file" ] || plan_die "no fix-keys.json in $plan_dir -- a reviewer mints the keys with mint-fix-keys.sh before a fix can be claimed" 66

# A claim for a pair nothing gates is a mistake, not a warning to step over:
# verify-fix-keys.sh reports it and carries on, so the real gated pair stays
# unclaimed while the mistake looks like progress.
#
# The gated pairs come from the review's Findings table, which is the source
# verify-fix-keys.sh and mint-fix-keys.sh both read. Reading fix-keys.json for
# them instead would let this writer accept a pair the verifier does not gate.
review_file="$plan_dir/adversarial-review.md"
[ -f "$review_file" ] || plan_die "no adversarial-review.md in $plan_dir -- there are no findings to claim a fix for" 66
if ! plan_review_gated_pairs "$review_file" \
        | grep -Fx "$(printf '%s\t%s' "$finding" "$work_unit")" >/dev/null; then
    plan_die "$finding/$work_unit is not a gated pair in the review's Findings table -- claim a pair the reviewer recorded, or add the finding and re-mint" 65
fi

# The key must be one mint wrote. Membership, not derivation: deriving it needs
# the minting session's secret, and a fixer able to read that secret could mint
# its own keys. A forged or stale key fails here without it.
grep -Fq "$key" "$keys_file" \
    || plan_die "that key is not in fix-keys.json -- it is forged, or stale from a previous minting; ask the reviewer for the key minted for $finding/$work_unit" 65

claims_file="$plan_dir/fixes.md"
claim="$(printf '%s\t%s\t%s' "$finding" "$work_unit" "$key")"
if [ -f "$claims_file" ] && grep -Fqx "$claim" "$claims_file"; then
    printf '%s: claim already recorded for %s/%s; nothing to do\n' "${0##*/}" "$finding" "$work_unit"
    exit 0
fi
# A second, different key for a pair already claimed would leave two lines and
# let verify pass on whichever it read first.
if [ -f "$claims_file" ] && awk -F'\t' -v f="$finding" -v w="$work_unit" \
        '$1 == f && $2 == w { found = 1 } END { exit !found }' "$claims_file"; then
    plan_die "$finding/$work_unit is already claimed with a different key in fixes.md -- remove that line, or re-mint the pair" 73
fi

# plan_atomic_write, not a hand-rolled per-PID temp file beside the target:
# MAINTAINER.md section 3 names it as the canonical helper, and
# test-duplication-ratchet.sh fails when the hand-rolled count grows -- which it
# did when this script first added one. The literal is left unspelled here on
# purpose: the ratchet counts occurrences in the source, so a comment quoting the
# pattern is counted as another use of it.
{
    [ ! -f "$claims_file" ] || cat "$claims_file"
    printf '%s\n' "$claim"
} | plan_atomic_write "$claims_file"

printf 'Recorded fix claim %s/%s in %s\n' "$finding" "$work_unit" "${claims_file##*/}"
printf '%s: verify with verify-fix-keys.sh --claimed-by <this session>, from a session that did not mint the keys\n' \
    "${0##*/}" >&2
