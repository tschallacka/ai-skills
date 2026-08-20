#!/usr/bin/env bash
plan_replace_paragraph() {
    local file="$1" paragraph_id="$2" content="$3" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    [[ "$content" != *$'\n\n'* ]] || plan_die "A paragraph replacement must contain exactly one paragraph; use section for multiple paragraphs"
    [ -n "$content" ] || plan_die "Paragraph content must not be empty"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v wanted="$paragraph_id" -v replacement="$content" '
        $0 == wanted {
            if (found++) exit 2
            print
            print replacement
            skipping = 1
            next
        }
        skipping && ($0 == "" || /^§[[:space:]][0-9]+\.[0-9]+$/ || /^## /) {
            if ($0 ~ /^§[[:space:]][0-9]+\.[0-9]+$/ || $0 ~ /^## /) print ""
            skipping = 0
        }
        !skipping { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Paragraph was not found exactly once: $paragraph_id"
    mv "$temporary_file" "$file"
    trap - RETURN
}
