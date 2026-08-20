#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# validate-plan-serve-lib.sh — the "the application still serves" advisory: a
# goal that changes module state, schema, or configuration must verify the
# running application, not just the changed artifact.
#
# Sourced by validate-plan.sh; never executed. Requires
# validate-plan-common-lib.sh and the inventory data model. Reads `skill_root`
# for the registry path.

# shellcheck disable=SC2154
# The entry script and its sibling libs publish these globals; shellcheck lints
# each file alone and cannot see the assignments.
set -euo pipefail

# --- "the application still serves": a goal that changes module state, schema,
#     or configuration must verify the running application, not just the changed
#     artifact. Registry-driven; a missing serve phrase is a WARN, not a gate.
state_change_registry="$skill_root/state-change-registry.json"

# Both nests must stay inside this one function: the indicator and serve-phrase
# scans use `break 2` to leave the unit loop from inside the phrase loop, which
# a split would turn into a silent no-op or an error.
plan_validate_still_serves() {
    [ -f "$state_change_registry" ] || return 0
    # PORTABILITY(mapfile): a read loop builds the same array on bash 3.2,
    # empty input included.
    state_indicators=()
    while IFS= read -r registry_line; do
        state_indicators+=("$registry_line")
    done < <(jq -r '.indicators[]?' "$state_change_registry" 2>/dev/null || true)
    serve_phrases=()
    while IFS= read -r registry_line; do
        serve_phrases+=("$registry_line")
    done < <(jq -r '.serve_phrases[]?' "$state_change_registry" 2>/dev/null || true)
    while IFS= read -r goal_name; do
        [ -n "$goal_name" ] || continue
        touched=false
        plan_map_load goal_units "$goal_name" || plan_map_value=""
        goal_unit_ids="$plan_map_value"
        for id in $goal_unit_ids; do
            plan_map_load unit_step "$id" || plan_map_value=""
            step_file="$plan_dir/$goal_name/steps/$plan_map_value.md"
            for ind in ${state_indicators[@]+"${state_indicators[@]}"}; do
                [ -n "$ind" ] || continue
                if grep -Fiq -- "$ind" "$step_file" 2>/dev/null; then touched=true; break 2; fi
                if grep -Fiq -- "$ind" "${step_file%.md}-testing.md" 2>/dev/null; then touched=true; break 2; fi
            done
        done
        [ "$touched" = true ] || continue
        served=false
        for id in $goal_unit_ids; do
            plan_map_load unit_type "$id" || plan_map_value=""
            case "$plan_map_value" in
                test|verification) ;;
                *) continue ;;
            esac
            plan_map_load unit_step "$id" || plan_map_value=""
            step_file="$plan_dir/$goal_name/steps/$plan_map_value.md"
            [ -f "$step_file" ] || continue
            acceptance="$(awk '
                /^## Acceptance criteria$/ { in_sec = 1; next }
                /^## / && in_sec { exit }
                in_sec { print }
            ' "$step_file")"
            if [ -f "${step_file%.md}-testing.md" ]; then
                acceptance="$acceptance
$(awk '
    /^## Automated tests$/ { in_sec = 1; next }
    /^## / && in_sec { exit }
    in_sec { print }
' "${step_file%.md}-testing.md")"
            fi
            # PORTABILITY(case-conversion)
            acceptance_lc="$(printf '%s' "$acceptance" | tr '[:upper:]' '[:lower:]')"
            for phr in ${serve_phrases[@]+"${serve_phrases[@]}"}; do
                [ -n "$phr" ] || continue
                phr_lc="$(printf '%s' "$phr" | tr '[:upper:]' '[:lower:]')"
                case "$acceptance_lc" in
                    *"$phr_lc"*) served=true; break 2 ;;
                esac
            done
        done
        if [ "$served" = false ]; then
            warn "$goal_name changes module state, schema, or configuration, but no verification acceptance condition checks that the application still serves; add one (e.g. a plain request returns HTTP 200)"
        fi
    done < <(plan_map_keys goal_units)
}
