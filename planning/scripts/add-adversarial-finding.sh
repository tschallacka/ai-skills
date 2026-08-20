#!/usr/bin/env bash
# MODE: PROD
# add-adversarial-finding.sh — append one row to a plan's adversarial-review.md
# "## Findings" table.
#
# The row is appended after the LAST existing row of the Findings table (the
# section runs from "## Findings" to the next "## " heading or EOF), which is
# the one anchor every other consumer agrees on: create-adversarial-review.sh
# seeds the table, update-adversarial-review.sh rewrites exactly that span, and
# mint-fix-keys.sh / verify-fix-keys.sh / validate-plan.sh all read it bounded
# by "## Verdict". No narrative sentence is used as an anchor.
#
# Naming a work unit gates the finding behind the fix-key mechanism, so the
# keys are re-minted for the plan; without one the cell stays N/A (ungated).
#
# Usage:
#   add-adversarial-finding.sh [--plan-dir] <plan-directory> <AR-NN> <finding> <resolution>
#       [open|in-progress|resolved] [--status <s>] [--work-unit <WNN>]
#   add-adversarial-finding.sh --help

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
# Accept --plan-dir as a synonym for the positional plan directory: the
# bounded reader takes the flag, so a reader who learned it there is not
# refused here.
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> <AR-NN> <finding> <resolution> [open|in-progress|resolved]
       ${0##*/} [--plan-dir] <plan-directory> <AR-NN> <finding> <resolution> [--status <status>] [--work-unit <WNN>]
       ${0##*/} --help

  --status <status>     open (default), in-progress, or resolved.
  --work-unit <WNN>     Gate the finding on a work unit; re-mints the plan's
                        fix keys. Omitted, the Work unit cell stays N/A.
USAGE
    exit "$rc"
}

status=""
work_unit=""
positional=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --status) [ "$#" -ge 2 ] || usage; status="$2"; shift 2 ;;
        --work-unit) [ "$#" -ge 2 ] || usage; work_unit="$2"; shift 2 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) positional+=("$1"); shift ;;
    esac
done
while [ "$#" -gt 0 ]; do positional+=("$1"); shift; done

set -- ${positional[@]+"${positional[@]}"}
if [ "$#" -lt 4 ] || [ "$#" -gt 5 ]; then usage; fi
plan_dir="$1" finding_id="$2" finding="$3" resolution="$4"
# The fifth positional is the legacy spelling of --status; --status wins.
[ -n "$status" ] || status="${5:-open}"
[ -n "$work_unit" ] || work_unit=N/A

plan_require_directory "$plan_dir"
plan_git_snapshot "$plan_dir"
# ^AR-[0-9]+$ is what mint-fix-keys.sh, verify-fix-keys.sh, validate-plan.sh
# and update-plan-content.sh all accept, so AR-1 must be addable here too.
[[ "$finding_id" =~ ^AR-[0-9]+$ ]] || plan_die 'Finding ID must use AR-NN'
[ "$work_unit" = N/A ] || [[ "$work_unit" =~ ^W[0-9]+$ ]] \
    || plan_die 'Work unit must be a work-unit ID such as W01'
plan_require_safe_value finding "$finding"
plan_require_safe_value resolution "$resolution"
case "$status" in
    open) status_cell='💤 open' ;;
    in-progress) status_cell='⏳ in progress' ;;
    resolved) status_cell='✅ resolved' ;;
    *) plan_die 'Finding status must be open, in-progress, or resolved' ;;
esac
review_file="$plan_dir/adversarial-review.md"
if [ ! -f "$review_file" ]; then
    printf '%s: %s\n' "${0##*/}" "Adversarial review not found: $review_file" >&2
    exit 66
fi
if grep -Eq "^\\|[[:space:]]*${finding_id}[[:space:]]*\\|" "$review_file"; then
    printf '%s: %s\n' "${0##*/}" "Finding already exists: $finding_id" >&2
    exit 73
fi

# The insertion line is the last table row of the Findings section. Computed in
# its own pass so the rewrite below stays a plain "insert after line N".
insert_after="$(awk '
    /^## Findings$/ { in_findings = 1; next }
    in_findings && /^## / { in_findings = 0 }
    in_findings && /^\|/ { last = NR }
    END { if (!last) exit 1; print last }
' "$review_file")" || plan_die "Review has no Findings table: $review_file"

temporary_file="${review_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
awk -v row="| $finding_id | $finding | $resolution | $status_cell | $work_unit |" \
    -v after="$insert_after" '
    { print }
    NR == after { print row; inserted = 1 }
    END { if (!inserted) exit 2 }
' "$review_file" > "$temporary_file" || plan_die 'Review has no finding insertion boundary'
mv "$temporary_file" "$review_file"
trap - EXIT
# A gated row without a key silently disables the whole fix-key gate, so mint
# now — the same thing update-adversarial-review.sh does after a rewrite.
if [ "$work_unit" != N/A ]; then
    "$script_dir/mint-fix-keys.sh" "$plan_dir" >&2
fi
printf 'Added %s\n' "$finding_id"
