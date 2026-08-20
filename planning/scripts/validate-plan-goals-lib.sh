#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# validate-plan-goals-lib.sh — the goal and step files: every goal.md carries
# the mandated headings and exactly one testing-requirement row, its work-unit
# count is inside the allowed band with an exception section when it is one,
# its testing requirement agrees with the test/verification units it owns, and
# every step file names its own goal, work unit, type, file, scope and subscope
# exactly as the inventory row does. A yes/no first-column table under a
# heading goal-tables.json does not register is hand-edit damage and fails.
#
# Sourced by validate-plan.sh; never executed. Requires
# validate-plan-common-lib.sh, jq, and the inventory data model.
#
# validate_goal_testing_requirement is the only writer of
# goal_testing_required, which is why the proof-coverage pass in
# validate-plan-inventory-lib.sh (running earlier) reads it empty.

# shellcheck disable=SC2154
# The entry script and its sibling libs publish these globals; shellcheck lints
# each file alone and cannot see the assignments.
set -euo pipefail

goal_tables_registry="$skill_root/goal-tables.json"
goal_registered_sections=''
goal_registered_list=''
goal_tables_loaded=false

# Registry membership decides which sections may carry a yes/no first column.
# The mutating helpers scan the testing-requirement section literally, so an
# unregistered table can only have arrived by hand and the document is suspect.
load_goal_table_registry() {
    [ "$goal_tables_loaded" = false ] || return 0
    goal_tables_loaded=true
    if [ ! -f "$goal_tables_registry" ]; then
        fail "goal-tables.json registry is missing at $goal_tables_registry"
        return 0
    fi
    goal_registered_sections="$(jq -r '.tables[] | select(.document == "goal.md") | .section' \
        "$goal_tables_registry" 2>/dev/null)"
    if [ -z "$goal_registered_sections" ]; then
        fail "goal-tables.json registers no goal.md section with a yes/no first column"
        return 0
    fi
    goal_registered_list="$(printf '%s' "$goal_registered_sections" | tr '\n' ',' | sed -e 's/,$//' -e 's/,/, /g')"
}

validate_goal_yes_no_tables() {
    local goal_file="$1" section newline
    newline=$'\n'
    while IFS= read -r section; do
        [ -n "$section" ] || continue
        case "$newline$goal_registered_sections$newline" in
            *"$newline$section$newline"*) ;;
            *) fail "$goal_file has a hand-written yes/no table under '$section'; only ${goal_registered_list:-none} may carry one. Rebuild that section through update-plan-content.sh -gs (or -gp for the one paragraph, -tr for the testing requirement), never by hand" ;;
        esac
    done < <(awk '
        /^```/ { in_fence = !in_fence; next }
        in_fence { next }
        /^## / { section = $0; next }
        /^\|[[:space:]]*(yes|no)[[:space:]]*\|/ {
            if (section != "" && !reported[section]++) print section
        }
    ' "$goal_file")
}

validate_goal_testing_requirement() {
    local goal_name="$1" goal_file="$2" row required rationale
    require_heading "$goal_file" '## Testing requirement'
    row="$(awk -F'|' '
        $0 == "## Testing requirement" { in_section = 1; next }
        in_section && /^## / { in_section = 0 }
        in_section && $0 == "| Test required | Rationale |" { headers++ }
        in_section && $0 == "|---|---|" { separators++ }
        in_section && /^\|[[:space:]]*(yes|no)[[:space:]]*\|[^|]+\|$/ {
            rows++
            value = $2; reason = $3
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", reason)
        }
        END {
            if (headers != 1 || separators != 1 || rows != 1) exit 2
            printf "%s\t%s\n", value, reason
        }
    ' "$goal_file")" || {
        fail "$goal_file must contain exactly one Test required/Rationale table row"
        plan_map_set goal_testing_required "$goal_name" ''
        return
    }
    IFS=$'\t' read -r required rationale <<< "$row"
    if [ -z "$rationale" ] || [[ "$rationale" == *'<'*'>'* ]]; then
        fail "$goal_file must explain why testing is or is not required"
    fi
    plan_map_set goal_testing_required "$goal_name" "$required"
}

plan_validate_goals() {
    load_goal_table_registry
    for goal_dir in "$plan_dir"/[0-9][0-9]-*/; do
        [ -d "$goal_dir" ] || continue
        goal_name="$(basename "$goal_dir")"
        goal_file="$goal_dir/goal.md"
        if [ ! -f "$goal_file" ]; then
            fail "Missing goal.md for $goal_name"
            continue
        fi
        for heading in \
            '## Current state and prior-goal handoffs' \
            '## Outcome and definition of done' \
            '## Why this goal is needed' \
            '## Scope' \
            '## Affected files, systems, data, and interfaces' \
            '## Dependencies and handoffs' \
            '## Implementation approach, risks, and edge cases' \
            '## Owned work units' \
            '## Testing requirement'; do
            require_heading "$goal_file" "$heading"
        done
        validate_goal_testing_requirement "$goal_name" "$goal_file"
        validate_goal_yes_no_tables "$goal_file"
        if grep -Fq '<required only when this goal has one permitted work unit>' "$goal_file"; then
            fail "$goal_name has an unfilled goal-size placeholder; fill the reason or remove the section"
        fi
        count=0
        plan_map_load goal_units "$goal_name" || plan_map_value=""
        goal_unit_ids="$plan_map_value"
        for id in $goal_unit_ids; do
            count=$((count + 1))
            grep -Fq "$id" "$goal_file" || fail "$goal_file does not name assigned work unit $id"
        done
        if [ "$count" -eq 0 ]; then
            fail "$goal_name has no assigned work units"
        fi
        if [ "$count" -eq 1 ]; then
            only_id="${goal_unit_ids# }"
            plan_map_load unit_type "$only_id" || plan_map_value=""
            only_type="$plan_map_value"
            case "$only_type" in
                docs|config|discovery|verification) ;;
                *) fail "$goal_name has one $only_type work unit; add its test/proof or merge it into its demonstrable outcome" ;;
            esac
            require_heading "$goal_file" '## Goal-size exception'
        elif [ "$count" -gt 10 ]; then
            fail "$goal_name has $count work units; split it at a stable outcome boundary"
        fi
        test_units=0
        for id in $goal_unit_ids; do
            plan_map_load unit_type "$id" || plan_map_value=""
            case "$plan_map_value" in
                test|verification) test_units=$((test_units + 1)) ;;
            esac
        done
        plan_map_load goal_testing_required "$goal_name" || plan_map_value=""
        goal_required="$plan_map_value"
        if [ "$goal_required" = yes ] && [ "$test_units" -eq 0 ]; then
            fail "$goal_name declares testing is required but has no test or verification work unit"
        elif [ "$test_units" -gt 0 ] && [ "$goal_required" != yes ]; then
            fail "$goal_name has a test or verification work unit but its testing requirement is not yes"
        fi
        if [ "$goal_required" = yes ]; then
            for id in $goal_unit_ids; do
                plan_map_load unit_type "$id" || plan_map_value=""
                [ "$plan_map_value" = docs ] && continue
                plan_map_load unit_step "$id" || plan_map_value=""
                companion="$plan_dir/$goal_name/steps/$plan_map_value-testing.md"
                [ -f "$companion" ] || fail "$id requires testing instructions at $companion"
            done
        fi
    done
}

plan_validate_step_files() {
    for id in ${unit_ids[@]+"${unit_ids[@]}"}; do
        plan_map_load unit_goal "$id" || plan_map_value=""
        u_goal="$plan_map_value"
        plan_map_load unit_step "$id" || plan_map_value=""
        u_step="$plan_map_value"
        plan_map_load unit_type "$id" || plan_map_value=""
        u_type="$plan_map_value"
        plan_map_load unit_file "$id" || plan_map_value=""
        u_file="$plan_map_value"
        plan_map_load unit_scope "$id" || plan_map_value=""
        u_scope="$plan_map_value"
        plan_map_load unit_subscope "$id" || plan_map_value=""
        u_subscope="$plan_map_value"
        goal_dir="$plan_dir/$u_goal"
        step_file="$goal_dir/steps/$u_step.md"
        if [ ! -d "$goal_dir" ]; then
            fail "$id references missing goal directory $u_goal"
            continue
        fi
        if [ ! -f "$step_file" ]; then
            fail "$id references missing step file $step_file"
            continue
        fi
        require_heading "$step_file" '## Ownership'
        require_heading "$step_file" '## Change target'
        require_heading "$step_file" '## Objective'
        require_heading "$step_file" '## Instructions'
        require_heading "$step_file" '## Acceptance criteria'
        require_heading "$step_file" '## Handoff'
        require_heading "$step_file" '## Atomicity check'
        grep -Fqx -- "- Goal: \`$u_goal\`" "$step_file" || fail "$step_file has wrong owning goal for $id"
        grep -Fqx -- "- Work unit: \`$id\`" "$step_file" || fail "$step_file has wrong work-unit ID"
        grep -Fqx -- "- Type: \`$u_type\`" "$step_file" || fail "$step_file has wrong type for $id"
        get_single_field "$step_file" 'File'; actual_file="$field_value"
        get_single_field "$step_file" 'Primary symbol or file scope'; actual_scope="$field_value"
        get_single_field "$step_file" 'Subscope'; actual_subscope="$field_value"
        [ "$actual_file" = "$u_file" ] || fail "$step_file file does not match $id inventory row"
        [ "$actual_scope" = "$u_scope" ] || fail "$step_file primary scope does not match $id inventory row"
        [ "$actual_subscope" = "$u_subscope" ] || fail "$step_file subscope does not match $id inventory row"
        grep -Fqx -- '- [x] This step owns exactly one inventory work unit.' "$step_file" || fail "$step_file has not confirmed one work unit"
        grep -Fqx -- '- [x] No other file, symbol, test target, or verification flow changes here.' "$step_file" || fail "$step_file has not confirmed target isolation"
        grep -Fqx -- '- [x] Any follow-on target has a separately named work unit and step.' "$step_file" || fail "$step_file has not confirmed follow-on ownership"
    done
}

plan_validate_step_naming() {
    for goal_dir in "$plan_dir"/[0-9][0-9]-*/; do
        [ -d "$goal_dir/steps" ] || continue
        goal_name="$(basename "$goal_dir")"
        while IFS= read -r step_file; do
            step_name="$(basename "$step_file" .md)"
            get_single_field "$step_file" 'Work unit'; declared_id="$field_value"
            if ! plan_map_has unit_type "$declared_id"; then
                fail "$step_file declares unlisted work unit '$declared_id'"
            elif { plan_map_load unit_goal "$declared_id" || plan_map_value=""; [ "$plan_map_value" != "$goal_name" ]; } \
                || { plan_map_load unit_step "$declared_id" || plan_map_value=""; [ "$plan_map_value" != "$step_name" ]; }; then
                fail "$step_file does not match the inventory assignment for $declared_id"
            fi
        done < <(find "$goal_dir/steps" -maxdepth 1 -type f -name '[0-9][0-9]-step-*.md' ! -name '*-testing.md' | sort)
        while IFS= read -r step_file; do
            step_name="$(basename "$step_file")"
            if [[ ! "$step_name" =~ ^[0-9][0-9]-step-[a-z0-9-]+\.md$ ]]; then
                fail "$step_file is not a numbered step file"
            fi
        done < <(find "$goal_dir/steps" -maxdepth 1 -type f -name '*.md' ! -name '*-testing.md' | sort)
    done
}
