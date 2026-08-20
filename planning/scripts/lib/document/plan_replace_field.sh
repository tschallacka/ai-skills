#!/usr/bin/env bash
plan_replace_field() {
    local file="$1" label="$2" value="$3" temporary_file
    plan_require_safe_value "$label" "$value"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v label="$label" -v replacement="$value" '
        $0 ~ "^- " label ":" {
            if (found++) exit 2
            print "- " label ": " replacement
            next
        }
        { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Field was not found exactly once: $label"
    mv "$temporary_file" "$file"
    trap - RETURN
}
