#!/usr/bin/env bash
# MODE: PROD
# resolve-finding.sh — close one adversarial-review finding: set its status,
# record its fix claim, and refuse a claim the minting session is making about
# its own keys.
#
# The three steps were previously three commands, and doing them by hand across
# eight review cycles produced two defects that the gate could not see (T54):
#
#   * A whole cycle's findings were reasoned about, remediated and cited by id
#     in six work units while never becoming rows, because the status flip was
#     done by regenerating the findings CSV *from* the table. Nine findings that
#     no row carried could not be minted, claimed, or reported as unclaimed —
#     verify-fix-keys iterates the pairs it finds, so it printed passed.
#   * Re-gating a finding onto a different unit left the old claim behind, and
#     verify-fix-keys reported four such rows as ignored rather than verified,
#     so fixes.md no longer said unambiguously which unit resolved them.
#
# Both are impossible through this command: it edits the row that exists rather
# than rewriting the table, and it removes a superseded claim for the same
# finding before recording the new one.
#
# Usage:
#   resolve-finding.sh [--plan-dir] <plan-directory> <AR-NN> [--status STATUS]
#                      [--claimed-by ID]
#   resolve-finding.sh --help
#
# The key comes from the plan's own fix-keys.json; the finding must already be
# gated on a work unit, because an ungated finding has no key to claim.
#
# Exit codes: 64 bad invocation, 65 the finding is absent or ungated,
# 66 the plan or its review file is missing, 70 self-certification refused.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> <AR-NN> [--status STATUS] [--claimed-by ID]
       ${0##*/} --help

  --status STATUS   the status to record (default: resolved)
  --claimed-by ID   the session recording the claim; refused when it equals the
                    session that minted the keys
USAGE
    exit "$rc"
}

# rjq reads the key out of fix-keys.json; refuse up front rather than half-way
# through, after the status has already been flipped.
command -v rjq >/dev/null 2>&1 || {
    printf '%s: rjq is required (it reads fix-keys.json); install rjq and re-run\n' "${0##*/}" >&2
    exit 69
}

case "${1:-}" in -h|--help) usage 0 ;; esac
[ "$#" -ge 2 ] || usage
plan_dir="$1"; finding="$2"; shift 2
new_status=resolved claimed_by=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --status)     [ "$#" -ge 2 ] || usage; new_status="$2"; shift 2 ;;
        --claimed-by) [ "$#" -ge 2 ] || usage; claimed_by="$2"; shift 2 ;;
        -h|--help)    usage 0 ;;
        *)            usage ;;
    esac
done

plan_require_directory "$plan_dir"
[[ "$finding" =~ ^AR-[0-9]+$ ]] || plan_die "Finding id must use AR-NN" 64
review_file="$plan_dir/adversarial-review.md"
[ -f "$review_file" ] || plan_die "adversarial-review.md not found: $review_file" 66
keys_file="$plan_dir/fix-keys.json"

# The work unit the finding is gated on, read from the row rather than supplied:
# a caller who names the unit can name the wrong one, and the row is the thing
# the mint, the claim and the verifier all agree on.
work_unit=''
while IFS= read -r line; do
    case "$line" in '|'*) ;; *) continue ;; esac
    [ "$(plan_table_cell "$line" 2)" = "$finding" ] || continue
    work_unit="$(plan_table_cell "$line" 6)"
    break
done < "$review_file"
[ -n "$work_unit" ] || plan_die "$finding has no row in $review_file; add it with add-adversarial-finding.sh before resolving it" 65
case "$work_unit" in
    W[0-9]*) ;;
    *) plan_die "$finding is not gated on a work unit (its cell reads '$work_unit'), so it has no key to claim" 65 ;;
esac

# Self-certification is refused here as well as at verification, so the refusal
# lands when the claim is made rather than at the gate that reads it later.
if [ -n "$claimed_by" ] && [ -f "$keys_file" ]; then
    minted_by="$(sed -n 's/.*"minted_by"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$keys_file" | head -1)"
    [ "$claimed_by" != "$minted_by" ] \
        || plan_die "refusing: $claimed_by minted these keys, so it cannot also claim them" 70
fi

plan_git_snapshot "$plan_dir"

# Flip the one row, in place. Rewriting the table from a regenerated CSV is what
# silently dropped nine findings; a row that exists is edited where it sits.
# mktemp rather than a temp path built from the target name and the pid: the
# duplication ratchet caps those sites, and nothing here needs the temp file to
# sit beside the document it rewrites.
temporary_file="$(mktemp "${TMPDIR:-/tmp}/resolve-finding.XXXXXX")"
plan_register_temp_file "$temporary_file"
before="$(plan_table_cell "$(grep -m1 "^| $finding " "$review_file")" 5)"
while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in '|'*) ;; *) printf '%s\n' "$line" >> "$temporary_file"; continue ;; esac
    if [ "$(plan_table_cell "$line" 2)" = "$finding" ]; then
        plan_table_set_cell "$line" 5 " $new_status " >> "$temporary_file"
    else
        printf '%s\n' "$line" >> "$temporary_file"
    fi
done < "$review_file"
mv "$temporary_file" "$review_file"

printf '%s: status %s -> %s (gated on %s)\n' "$finding" "$before" "$new_status" "$work_unit"

# The claim, from the plan's own key. A superseded claim for the same finding is
# removed first: leaving it is what made verify-fix-keys report ignored rather
# than verified, so the file stopped saying which unit resolved the finding.
[ -f "$keys_file" ] || { printf '%s: no fix-keys.json yet; mint keys before claiming\n' "${0##*/}" >&2; exit 0; }
key="$(rjq -r --arg f "$finding" --arg w "$work_unit" '.keys[$f][$w] // empty' "$keys_file" 2>/dev/null || true)"
[ -n "$key" ] || { printf '%s: no key for %s/%s; re-mint after adding the row\n' "${0##*/}" "$finding" "$work_unit" >&2; exit 0; }

fixes="$plan_dir/fixes.md"
if [ -f "$fixes" ] && grep -q "^$finding	" "$fixes" 2>/dev/null; then
    superseded="$(grep -c "^$finding	" "$fixes" || true)"
    pruned="$(mktemp "${TMPDIR:-/tmp}/resolve-fixes.XXXXXX")"
    plan_register_temp_file "$pruned"
    grep -v "^$finding	" "$fixes" > "$pruned" && mv "$pruned" "$fixes"
    printf '%s: dropped %s superseded claim row(s) for %s\n' "${0##*/}" "$superseded" "$finding" >&2
fi
"$script_dir/add-fix-claim.sh" "$plan_dir" --finding "$finding" --work-unit "$work_unit" --key "$key"
