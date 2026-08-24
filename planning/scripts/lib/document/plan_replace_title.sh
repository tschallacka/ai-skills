#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_replace_title() {
    local file="$1" title="$2" temporary_file rc
    plan_require_safe_value title "$title"
    [[ "$title" != *$'\n'* ]] || plan_die "Title must be one line"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
    trap 'rm -f "$temporary_file"' RETURN
    rc=0
    awk -v replacement="$title" '
        /^# / {
            if (found++) exit 2
            if ($0 !~ /:/) exit 3
            sub(/:.*/, ": " replacement)
            print
            next
        }
        { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || rc=$?
    # A rename that changes nothing must not read as success: exit 3 is the
    # damaged-heading case, where there is no ": title" part to rewrite.
    case $rc in
        0) ;;
        3) plan_die "Document title heading has no ': title' part to replace" ;;
        *) plan_die "Document title was not found exactly once" ;;
    esac
    mv "$temporary_file" "$file"
    trap - RETURN
}
