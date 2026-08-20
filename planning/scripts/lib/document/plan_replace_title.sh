#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
plan_replace_title() {
    local file="$1" title="$2" temporary_file
    plan_require_safe_value title "$title"
    [[ "$title" != *$'\n'* ]] || plan_die "Title must be one line"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v replacement="$title" '
        /^# / {
            if (found++) exit 2
            sub(/:.*/, ": " replacement)
            print
            next
        }
        { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Document title was not found exactly once"
    mv "$temporary_file" "$file"
    trap - RETURN
}
