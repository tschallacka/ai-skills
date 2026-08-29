#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_replace_testing_requirement() {
    local file="$1" required="$2" rationale="$3" replacement temporary_file
    case "$required" in
        yes|no) ;;
        *) plan_die "Test requirement must be yes or no" ;;
    esac
    plan_require_safe_value rationale "$rationale"
    replacement="| $required | $rationale |"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v replacement="$replacement" '
        $0 == "## Testing requirement" {
            in_section = 1
            print
            next
        }
        in_section && /^## / {
            in_section = 0
        }
        in_section && $0 == "| Test required | Rationale |" {
            header = 1
            print
            next
        }
        in_section && header && /^\|---\|---\|$/ {
            separator = 1
            print
            next
        }
        in_section && separator && /^\|[^|]+\|[^|]+\|$/ {
            if (data_row++) exit 3
            print replacement
            next
        }
        { print }
        END {
            if (!header || !separator || data_row != 1) exit 2
        }
    ' "$file" > "$temporary_file" || plan_die "Testing requirement table was not found exactly once: $file"
    mv "$temporary_file" "$file"
    trap - RETURN
}
