#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 4 ] || [ "$#" -gt 5 ]; then
    printf 'Usage: %s <plan-directory> <AR-NN> <finding> <resolution> [open|in-progress|resolved]\n' "$(basename "$0")" >&2
    exit 64
fi

plan_dir="$1" finding_id="$2" finding="$3" resolution="$4" status="${5:-open}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
plan_require_directory "$plan_dir"
plan_git_snapshot "$plan_dir"
[[ "$finding_id" =~ ^AR-[0-9][0-9]+$ ]] || plan_die 'Finding ID must use AR-NN'
plan_require_safe_value finding "$finding"
plan_require_safe_value resolution "$resolution"
case "$status" in
    open) status_cell='💤 open' ;;
    in-progress) status_cell='⏳ in progress' ;;
    resolved) status_cell='✅ resolved' ;;
    *) plan_die 'Finding status must be open, in-progress, or resolved' ;;
esac
review_file="$plan_dir/adversarial-review.md"
[ -f "$review_file" ] || plan_die "Adversarial review not found: $review_file"
! grep -Eq "^\\|[[:space:]]*$finding_id[[:space:]]*\\|" "$review_file" || plan_die "Finding already exists: $finding_id"

temporary_file="${review_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
awk -v row="| $finding_id | $finding | $resolution | $status_cell | N/A |" '
    /^No additional substantive finding remains\./ && !inserted { print row; print ""; inserted=1 }
    { print }
    END { if (!inserted) exit 2 }
' "$review_file" > "$temporary_file" || plan_die 'Review has no finding insertion boundary'
mv "$temporary_file" "$review_file"
trap - EXIT
printf 'Added adversarial finding %s\n' "$finding_id"
