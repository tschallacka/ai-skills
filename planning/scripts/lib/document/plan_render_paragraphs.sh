#!/usr/bin/env bash
plan_render_paragraphs() {
    local number="$1" content="$2"
    [ -n "$content" ] || plan_die "Section content must not be empty"
    printf '%s' "$content" | awk -v number="$number" '
        BEGIN { RS=""; ORS="" }
        {
            text = $0
            # No "\n" inside the bracket expression: a backslash escape there
            # is undefined in POSIX awk, and [[:space:]] already covers newline.
            sub(/^[[:space:]]+/, "", text)
            sub(/[[:space:]]+$/, "", text)
            if (text == "") next
            if (count++) printf "\n\n"
            printf "§ %s.%d\n%s", number, count, text
        }
        END { if (count == 0) exit 1 }
    '
}
