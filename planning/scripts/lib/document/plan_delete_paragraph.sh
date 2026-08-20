#!/usr/bin/env bash
# Delete one numbered paragraph and renumber the rest of its section so labels
# stay sequential. Targeted rather than re-emitting the section, which risks a
# transcription slip damaging paragraphs no finding was about.
plan_delete_paragraph() {
    local file="$1" paragraph_id="$2" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v wanted="$paragraph_id" '
        BEGIN {
            target_value = wanted
            sub(/^§ /, "", target_value)
            split(target_value, target_parts, /\./)
            target_section = target_parts[1]
            target_number = target_parts[2] + 0
        }
        /^§ [0-9]+\.[0-9]+$/ {
            current_value = $0
            sub(/^§ /, "", current_value)
            split(current_value, current_parts, /\./)
            section = current_parts[1]
            number = current_parts[2] + 0
            if (section == target_section && number == target_number) {
                if (target_found++) exit 2
                skipping = 1
                next
            }
            skipping = 0
            if (section == target_section && number > target_number) {
                print "§ " section "." (number - 1)
            } else {
                print
            }
            next
        }
        /^## / {
            # A section boundary always stops the delete: never swallow a
            # following heading even when the deleted paragraph was the last in
            # its section (that would re-parent the next section under it).
            skipping = 0
            print
            next
        }
        skipping { next }
        { print }
        END { if (target_found != 1) exit 3 }
    ' "$file" > "$temporary_file" || plan_die "Paragraph was not found exactly once: $paragraph_id"
    mv "$temporary_file" "$file"
    trap - RETURN
}
