#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# The whole row, for a caller that rewrites the region around it. Scoped to the
# section on purpose: a yes/no table elsewhere in the goal is a different table.
plan_testing_requirement_row() {
    local goal_file="$1"
    awk '
        $0 == "## Testing requirement" { in_section = 1; next }
        in_section && /^## / { exit }
        in_section && /^\|[[:space:]]*(yes|no)[[:space:]]*\|/ { print; exit }
    ' "$goal_file"
}
