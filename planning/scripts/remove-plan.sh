#!/usr/bin/env bash
# remove-plan.sh — remove a plan directory and reconcile the plans-root git
# history.
#
# Usage:
#   remove-plan.sh <plan-directory>
#
# Removes the plan directory (plan-description.md must exist, so a stray path
# is not destroyed). When the enclosing plans root then holds no other plan
# directories, the root's own git history (created by create-plan.sh when the
# root is git-excluded or outside any repo) is cleared so a discarded
# initiative does not leave stale history behind. The root directory itself
# and its .env manifest are preserved; the next create-plan.sh re-initializes
# the history.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/plan-document-lib.sh"

usage() {
    printf 'Usage: %s <plan-directory>\n' "$(basename "$0")" >&2
    exit 64
}

[ "$#" -eq 1 ] || usage
plan_dir="$1"
plan_require_directory "$plan_dir"
[ -f "$plan_dir/plan-description.md" ] || plan_die "not a plan directory (no plan-description.md): $plan_dir"

root="$(cd "$plan_dir/.." && pwd -P)"
rm -rf "$plan_dir"
printf 'Removed plan %s\n' "$plan_dir"

# If the enclosing root still holds another plan, leave it (and its history)
# alone. Otherwise, if the root carries its own .git (the create-plan.sh
# git-excluded/outside-repo case), clear that history so no stale plan commit
# survives the last plan.
remaining="$(find "$root" -mindepth 2 -maxdepth 2 -name plan-description.md -print -quit 2>/dev/null || true)"
if [ -n "$remaining" ]; then
    exit 0
fi
if [ -d "$root/.git" ]; then
    rm -rf "$root/.git"
    printf 'Cleared plans-root git history at %s (no plans remain)\n' "$root"
fi