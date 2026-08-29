#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# Derive a row description from a goal's "## Outcome and definition of done",
# skipping "§ N.N" labels and truncating to 100 chars. Falls back to "$2" so a
# plan-level tracker never carries a literal placeholder.
plan_goal_definition_of_done() {
    local goal_file="$1" fallback="$2"
    local desc
    desc="$(awk '
        /^## Outcome and definition of done$/ { in_sec = 1; next }
        in_sec && /^## / { exit }
        in_sec && /^§ [0-9]+\.[0-9]+[[:space:]]*$/ { next }
        in_sec && NF {
            line = $0; sub(/^[[:space:]]+/, "", line)
            if (length(line) > 100) line = substr(line, 1, 100) "..."
            print line; exit
        }
    ' "$goal_file" 2>/dev/null)"
    [ -n "$desc" ] || desc="$fallback"
    printf '%s\n' "$desc"
}
