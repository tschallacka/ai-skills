#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
plan_testing_requirement_for_goal() {
    local goal_file="$1"
    awk -F'|' '
        $0 == "## Testing requirement" { in_section = 1; next }
        in_section && /^## / { exit }
        in_section && /^\|[[:space:]]*(yes|no)[[:space:]]*\|/ {
            value = $2
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
            print value
            exit
        }
    ' "$goal_file"
}
