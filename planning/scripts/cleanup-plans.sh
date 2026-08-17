#!/usr/bin/env bash
# cleanup-plans.sh — list completed plans and remove selected ones.
#
# Usage:
#   cleanup-plans.sh [--list] [<plan-name> ...] [--yes]
#
# With no plan names: lists every plan under the resolved plans root and marks
# the completed ones (plan-level progress.md at 100%). With plan names: removes
# each named plan after confirmation, using remove-plan.sh (which clears the
# plans-root git history when the last plan is removed). --yes skips the
# confirmation prompt for non-interactive runs.
#
# Plan names are matched against plan directory names (kebab-case). An unknown
# name is an error, not a silent skip, so a typo cannot remove the wrong plan.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/plan-document-lib.sh"

usage() {
    printf 'Usage: %s [--list] [<plan-name> ...] [--yes]\n' "$(basename "$0")" >&2
    exit 64
}

yes_flag=false
list_only=false
wanted=()
for arg in "$@"; do
    case "$arg" in
        -h|--help) usage ;;
        --yes|-y) yes_flag=true ;;
        --list|-l) list_only=true ;;
        --*) usage ;;
        *) wanted+=("$arg") ;;
    esac
done

plans_root="$(plan_default_root)"
[ -n "${PLANS_ROOT:-}" ] && plans_root="${PLANS_ROOT%/}"
[ -d "$plans_root" ] || { printf 'cleanup-plans: plans root not found: %s\n' "$plans_root" >&2; exit 66; }

plan_is_complete() {
    local plan_dir="$1"
    [ -f "$plan_dir/progress.md" ] || return 1
    grep -Fq '**Overall progress:** `100%' "$plan_dir/progress.md"
}

# All plan directories (directories containing plan-description.md), sorted.
mapfile -t plan_dirs < <(find "$plans_root" -mindepth 2 -maxdepth 2 -name plan-description.md -printf '%h\n' 2>/dev/null | sort -u)

if [ "${#plan_dirs[@]}" -eq 0 ]; then
    printf 'cleanup-plans: no plans under %s\n' "$plans_root"
    exit 0
fi

# Resolve plan names to directories; reject unknown names.
targets=()
if [ "${#wanted[@]}" -eq 0 ]; then
    targets=("${plan_dirs[@]}")
else
    for name in "${wanted[@]}"; do
        found=""
        for dir in "${plan_dirs[@]}"; do
            [ "$(basename "$dir")" = "$name" ] || continue
            [ -f "$dir/plan-description.md" ] || continue
            found="$dir"
            break
        done
        [ -n "$found" ] || { printf 'cleanup-plans: no plan named %s under %s\n' "$name" "$plans_root" >&2; exit 66; }
        targets+=("$found")
    done
fi

if [ "$list_only" = true ]; then
    printf 'Plans under %s:\n' "$plans_root"
    for dir in "${targets[@]}"; do
        if plan_is_complete "$dir"; then
            printf '  ✅ %s  (completed)\n' "$(basename "$dir")"
        else
            printf '  💤 %s\n' "$(basename "$dir")"
        fi
    done
    exit 0
fi

# Completed candidates are the removable set; an explicitly named incomplete
# plan is still listed so the caller can decide (the AI confirms with the user).
printf 'The following plans would be removed:\n'
for dir in "${targets[@]}"; do
    state='💤 incomplete'
    plan_is_complete "$dir" && state='✅ completed'
    printf '  %s  %s\n' "$state" "$(basename "$dir")"
done

if [ "$yes_flag" = false ]; then
    printf 'Proceed? [y/N] ' >&2
    if [ -t 0 ]; then
        read -r answer
    else
        answer=""
    fi
    case "$answer" in
        y|Y|yes) ;;
        *) printf 'cleanup-plans: cancelled\n'; exit 1 ;;
    esac
fi

for dir in "${targets[@]}"; do
    "$SCRIPT_DIR/remove-plan.sh" "$dir"
done
printf 'cleanup-plans: removed %d plan(s)\n' "${#targets[@]}"