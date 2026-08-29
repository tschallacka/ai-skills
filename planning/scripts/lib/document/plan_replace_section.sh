#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_replace_section() {
    local file="$1" heading="$2" body_file="$3" temporary_file
    plan_refuse_field_section "$file" "$heading"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v heading="$heading" -v replacement="$body_file" '
        BEGIN {
            while ((getline line < replacement) > 0) {
                body = body (body == "" ? "" : "\n") line
            }
            close(replacement)
        }
        $0 == heading {
            if (found++) exit 2
            print
            print ""
            print body
            skipping = 1
            next
        }
        skipping && /^## / { skipping = 0; print "" }
        !skipping { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "$(plan_missing_section_message "$file" "$heading")"
    mv "$temporary_file" "$file"
    trap - RETURN
}
