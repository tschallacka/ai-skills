#!/usr/bin/env bash
# validate-plan-inventory-lib.sh — the work-unit inventory: parse every row,
# enforce the per-row rules (ID shape, type vocabulary, one file, one symbol,
# selector shapes, goal/step names), cross-link the definition-of-done
# coverage table, walk the dependency graph for cycles and unknown edges, and
# run the proof-coverage rule.
#
# Sourced by validate-plan.sh; never executed. Requires
# validate-plan-common-lib.sh.
#
# The data model every later pass reads lives here, keyed by work-unit ID:
# unit_type, unit_file, unit_scope, unit_subscope, unit_goal, unit_step,
# unit_depends, plus unit_ids (declaration order), goal_units (goal -> ID list)
# and goal_testing_required (written later, by the goals pass).
#
# The maps are plan_map_* (bash 3.2 has no associative arrays), so they are
# process-global and need no declaration.
# shellcheck disable=SC2154
# The entry script and its sibling libs publish these globals; shellcheck lints
# each file alone and cannot see the assignments.
set -euo pipefail

# The inventory maps. plan_map_* instead of `declare -A`, which is bash 4 and
# aborts on stock macOS -- validate-plan.sh is the plan gate, so it has to run
# on the floor. Keys are work-unit ids and goal directory names.
unit_ids=()

plan_validate_inventory() {
    while IFS=$'\t' read -r id type file scope subscope intended depends goal step; do
        [ -n "$id" ] || continue
        if [[ ! "$id" =~ ^W[0-9][0-9]+$ ]]; then
            fail "Invalid work-unit ID: $id"
            continue
        fi
        if plan_map_has unit_type "$id"; then
            fail "Duplicate work-unit ID: $id"
            continue
        fi
        case "$type" in
            source|markup|style|test|config|docs|data|generated|discovery|verification) ;;
            *) fail "$id has unsupported type '$type'" ;;
        esac
        if [ -z "$file" ] || [ -z "$scope" ] || [ -z "$subscope" ] || [ -z "$intended" ] || [ -z "$goal" ] || [ -z "$step" ]; then
            fail "$id has an empty required work-unit field"
        fi
        if [ "$type" = verification ] && [ "$file" != N/A ]; then
            fail "$id is verification and must use File 'N/A'"
        fi
        if [ "$type" != verification ] && [ "$file" = N/A ]; then
            fail "$id is not verification and must name one file"
        fi
        if [[ "$file" == *'*'* || "$file" == */ ]]; then
            fail "$id must name one concrete file, not a glob or directory: $file"
        fi
        # A scope names one symbol. Key on the count of ::-qualified symbols,
        # not on conjunctions: " and " legitimately joins one file's own
        # description. A comma list still signals multiple scopes.
        if [ "$type" != verification ]; then
            sym_count="$(printf '%s' "$scope" | grep -oE '[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*(\(\))?' | wc -l | tr -d ' ')" || true
            if [ "$sym_count" -gt 1 ]; then
                fail "$id lists multiple symbols or scopes: $scope"
            elif [[ "$scope" == *','* ]]; then
                fail "$id lists multiple symbols or scopes: $scope"
            fi
        fi
        if [ "$type" = style ] && [[ ! "$scope" =~ ^[.#][A-Za-z_-][A-Za-z0-9_-]*$ ]]; then
            fail "$id style scope must be one CSS selector, such as .completion-message"
        fi
        if [ "$type" = markup ] && [[ ! "$scope" =~ ^[#.][A-Za-z_-][A-Za-z0-9_-]*$ ]]; then
            fail "$id markup scope must be one named DOM selector, such as #checkout-summary"
        fi
        if [[ ! "$goal" =~ ^[0-9][0-9]-[a-z0-9-]+$ ]]; then
            fail "$id has invalid goal name '$goal'"
        fi
        if [[ ! "$step" =~ ^[0-9][0-9]-step-[a-z0-9-]+$ ]]; then
            fail "$id has invalid step name '$step'"
        fi
        if [ "$subscope" != N/A ] && { [[ "$subscope" == *','* ]] || [[ "$subscope" == *' and '* ]]; }; then
            fail "$id lists multiple subscope targets: $subscope"
        fi
        plan_map_set unit_type "$id" "$type"
        plan_map_set unit_file "$id" "$file"
        plan_map_set unit_scope "$id" "$scope"
        plan_map_set unit_subscope "$id" "$subscope"
        plan_map_set unit_goal "$id" "$goal"
        plan_map_set unit_step "$id" "$step"
        plan_map_set unit_depends "$id" "$depends"
        unit_ids+=("$id")
        if plan_map_has seen_steps "$goal/$step"; then
            fail "Multiple work units are assigned to $goal/steps/$step.md"
        fi
        plan_map_set seen_steps "$goal/$step" "$id"
        # goal_units accumulates a space-delimited id list, leading space and all
        # (callers word-split it and one caller strips the leading space).
        plan_map_load goal_units "$goal" || plan_map_value=""
        plan_map_set goal_units "$goal" "$plan_map_value $id"
    done < <(
        awk -F'|' '
            /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
                for (i = 2; i <= 10; i++) {
                    value = $i
                    gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
                    gsub(/^`|`$/, "", value)
                    printf "%s%s", value, (i == 10 ? ORS : "\t")
                }
            }
        ' "$inventory"
    )

    if [ "${#unit_ids[@]}" -eq 0 ]; then
        fail "No work-unit rows found; use IDs such as W01 in the Work units table"
    fi

    while IFS= read -r coverage_id; do
        [ -n "$coverage_id" ] && plan_map_set coverage_ids "$coverage_id" 1
    done < <(
        awk -F'|' '
            /^## Work units/ { exit }
            /^\|/ && $2 !~ /Required outcome/ && $2 !~ /^-+$/ {
                print $3
            }
        ' "$inventory" | grep -oE 'W[0-9][0-9]+' || true
    )
    if [ "$(plan_map_count coverage_ids)" -eq 0 ]; then
        fail "Definition-of-done coverage has no work-unit references"
    fi
    for id in ${unit_ids[@]+"${unit_ids[@]}"}; do
        plan_map_has coverage_ids "$id" || fail "$id is not linked to a definition-of-done item"
    done
    while IFS= read -r coverage_id; do
        [ -n "$coverage_id" ] || continue
        plan_map_has unit_type "$coverage_id" || fail "Definition-of-done coverage names unknown work unit $coverage_id"
    done < <(plan_map_keys coverage_ids)
}

# Recursive: do not rename or inline (it calls itself, and the visit_state
# tri-colour marking is what turns the recursion into cycle detection).
check_dependencies() {
    local id="$1" dependency
    plan_map_load visit_state "$id" || plan_map_value=""
    case "$plan_map_value" in
        visiting) fail "Dependency cycle includes $id"; return ;;
        done) return ;;
    esac
    plan_map_set visit_state "$id" visiting
    while IFS= read -r dependency; do
        [ -n "$dependency" ] || continue
        if ! plan_map_has unit_type "$dependency"; then
            fail "$id depends on unknown work unit $dependency"
        else
            check_dependencies "$dependency"
        fi
    done < <(plan_map_load unit_depends "$id" || true; printf '%s\n' "$plan_map_value" | grep -oE 'W[0-9][0-9]+' || true)
    plan_map_set visit_state "$id" 'done'
}

plan_validate_dependency_graph() {
    for id in ${unit_ids[@]+"${unit_ids[@]}"}; do
        check_dependencies "$id"
    done
}

# Recursive: do not rename. `proof_seen` is the memo that stops it looping on a
# cyclic graph; the caller resets it before each root query.
depends_on() {
    local candidate="$1" required="$2" dependency key
    [ "$candidate" = "$required" ] && return 0
    key="$candidate/$required"
    plan_map_has proof_seen "$key" && return 1
    plan_map_set proof_seen "$key" 1
    while IFS= read -r dependency; do
        [ -n "$dependency" ] || continue
        [ "$dependency" = "$required" ] && return 0
        depends_on "$dependency" "$required" && return 0
    done < <(plan_map_load unit_depends "$candidate" || true; printf '%s\n' "$plan_map_value" | grep -oE 'W[0-9][0-9]+' || true)
    return 1
}

# KNOWN DEAD, deliberately: this pass acts only when goal_testing_required is
# `yes`, and that map is populated by a pass the entry script runs LATER, so it
# is always empty here. Reordering would activate an untested gate.
plan_validate_proof_coverage() {
    for id in ${unit_ids[@]+"${unit_ids[@]}"}; do
        plan_map_load unit_type "$id" || plan_map_value=""
        case "$plan_map_value" in
            source|markup|style|config|data|generated)
                plan_map_load unit_goal "$id" || plan_map_value=""
                plan_map_load goal_testing_required "$plan_map_value" || plan_map_value=""
                [ "$plan_map_value" = yes ] || continue
                has_proof=false
                for proof_id in ${unit_ids[@]+"${unit_ids[@]}"}; do
                    plan_map_load unit_type "$proof_id" || plan_map_value=""
                    case "$plan_map_value" in
                        test|verification)
                            plan_map_clear proof_seen
                            if depends_on "$proof_id" "$id"; then
                                has_proof=true
                                break
                            fi
                            ;;
                    esac
                done
                [ "$has_proof" = true ] || fail "$id has no downstream test or verification work unit"
                ;;
        esac
    done
}
