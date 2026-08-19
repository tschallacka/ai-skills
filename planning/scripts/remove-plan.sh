#!/usr/bin/env bash
# remove-plan.sh — remove a plan directory and reconcile the plans-root git
# history.
#
# Usage:
#   remove-plan.sh [--plan-dir] <plan-directory>
#   remove-plan.sh --help
#
# Removes the plan directory (plan-description.md must exist, so a stray path
# is not destroyed). When the enclosing plans root then holds no other plan
# directories, the root's own git history (created by create-plan.sh when the
# root is git-excluded or outside any repo) is cleared so a discarded
# initiative does not leave stale history behind. The root directory itself
# and its .env manifest are preserved; the next create-plan.sh re-initializes
# the history.
#
# Exit codes: 64 bad invocation or not a plan directory, 66 no such directory.

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
Usage: ${0##*/} [--plan-dir] <plan-directory>
       ${0##*/} --help
USAGE
    exit "$rc"
}

plan_dir=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) [ -z "$plan_dir" ] || usage; plan_dir="$1"; shift ;;
    esac
done
[ -n "$plan_dir" ] || usage

plan_require_directory "$plan_dir"
[ -f "$plan_dir/plan-description.md" ] || plan_die "not a plan directory (no plan-description.md): $plan_dir"

root="$(cd "$plan_dir/.." && pwd -P)"
rm -rf "$plan_dir"

# Clear the root's own history only once the last plan is gone, so no stale
# plan commit survives it; a root still holding another plan keeps its history.
remaining="$(find "$root" -mindepth 2 -maxdepth 2 -name plan-description.md -print -quit 2>/dev/null || true)"
if [ -z "$remaining" ] && [ -d "$root/.git" ]; then
    rm -rf "$root/.git"
    # Diagnostic, not the result: stdout carries exactly one line (§10).
    printf '%s: cleared plans-root git history at %s (no plans remain)\n' "${0##*/}" "$root" >&2
fi

printf 'Removed plan %s\n' "$plan_dir"
