#!/usr/bin/env bash
# validate-plan-placeholders-lib.sh — the template-placeholder sweep: registry
# membership decides what is a placeholder, and the registered surface
# (authored | generated) decides whether the finding is a WARN or a FAIL.
#
# Sourced by validate-plan.sh; never executed. Requires
# validate-plan-common-lib.sh and the `plan_docs` list from
# validate-plan-docs-lib.sh. Reads `skill_root` for the registry path.

# shellcheck disable=SC2154
# The entry script and its sibling libs publish these globals; shellcheck lints
# each file alone and cannot see the assignments.
set -euo pipefail

# Template-placeholder check: detection is LITERAL membership in
# planning/placeholders.json, so author-written <...> prose is never flagged.
# Each entry's surface (authored | generated) drives the verdict.
registry_file="$skill_root/placeholders.json"
placeholder_re='<[^<>]*[A-Za-z][^<>]*>'

# Print the registry surface for a token; empty if not registered.
token_surface() {
    [ -f "$registry_file" ] || return 1
    jq -r --arg t "$1" '.placeholders[] | select(.token == $t) | .surface' "$registry_file" 2>/dev/null
}

# Scan one file for registered placeholders and apply the surface-based verdict.
#   check_placeholders <authored|generated> <label> <file>
check_placeholders() {
    local surface_arg="$1" label="$2" file="$3" tok surf
    [ -f "$file" ] || return 0
    while IFS= read -r tok; do
        surf="$(token_surface "$tok")"
        [ -n "$surf" ] || continue   # not registered -> author prose, ignore
        if [ "$surface_arg" = generated ]; then
            fail "$label contains a registered placeholder that no author will fill: $tok"
        elif [ "$complete_mode" = true ]; then
            fail "$label still contains a registered placeholder: $tok"
        else
            warn "$label contains a registered placeholder (fill before completion): $tok"
        fi
    done < <(awk -v re="$placeholder_re" '
        /^```/ { in_fence = !in_fence; next }
        !in_fence {
            while (match($0, re)) {
                print substr($0, RSTART, RLENGTH)
                $0 = substr($0, RSTART + RLENGTH)
            }
        }
    ' "$file" | sort -u)
}

plan_validate_placeholders() {
    # Authored documents: plan narrative, review, inventory, goals, and steps.
    for doc in "${plan_docs[@]}"; do
        [ -f "$doc" ] || continue
        check_placeholders authored "$(basename "$doc")" "$doc"
    done
    # Generated artifacts: plan progress tracker.
    [ -f "$plan_dir/progress.md" ] && check_placeholders generated "plan progress.md" "$plan_dir/progress.md"
    # Generated artifacts: goal progress trackers and goal-size-exception sections.
    for goal_dir in "$plan_dir"/[0-9][0-9]-*/; do
        [ -d "$goal_dir" ] || continue
        [ -f "$goal_dir/progress.md" ] && check_placeholders generated "$(basename "$goal_dir") progress.md" "$goal_dir/progress.md"
        [ -f "$goal_dir/goal.md" ] || continue
        while IFS= read -r tok; do
            surf="$(token_surface "$tok")"
            [ -n "$surf" ] || continue
            fail "$(basename "$goal_dir") goal-size exception contains a registered placeholder that no author will fill: $tok"
        done < <(awk -v re="$placeholder_re" '
            /^## Goal-size exception/ { in_sec = 1; next }
            in_sec && /^## / && !/^## Goal-size exception/ { exit }
            in_sec && /^```/ { in_fence = !in_fence; next }
            !in_fence && in_sec {
                while (match($0, re)) {
                    print substr($0, RSTART, RLENGTH)
                    $0 = substr($0, RSTART + RLENGTH)
                }
            }
        ' "$goal_dir/goal.md" | sort -u)
    done
    # Generated artifacts: UI-story run caches.
    if [ -d "$plan_dir/ui-story-runs" ]; then
        while IFS= read -r cache_file; do
            check_placeholders generated "$(basename "$cache_file") run cache" "$cache_file"
        done < <(find "$plan_dir/ui-story-runs" -maxdepth 1 -name '*.md' -type f 2>/dev/null)
    fi
}
