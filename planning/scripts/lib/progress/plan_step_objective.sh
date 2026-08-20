#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# Derive a row description from a step's Objective paragraph: the text after
# the first "§ N.N" label inside "## Objective", truncated to 100 chars. Falls
# back to "$2" so a progress table never carries a literal placeholder.
plan_step_objective() {
    local step_file="$1" fallback="$2"
    local desc
    desc="$(awk '
        /^## Objective$/ { in_obj = 1; next }
        /^§ [0-9]+\.[0-9]+$/ && in_obj { after_label = 1; next }
        after_label && NF {
            line = $0; sub(/^[[:space:]]+/, "", line)
            if (length(line) > 100) line = substr(line, 1, 100) "..."
            print line; exit
        }
        /^## / && in_obj { exit }
    ' "$step_file" 2>/dev/null)"
    [ -n "$desc" ] || desc="$fallback"
    printf '%s\n' "$desc"
}
