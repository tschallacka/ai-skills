#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_insert_paragraph() {
    local file="$1" paragraph_id="$2" mode="$3" body_file="$4" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    case "$mode" in before|after) ;; *) plan_die "Paragraph insertion mode must be before or after" ;; esac
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v wanted="$paragraph_id" -v mode="$mode" -v body_file="$body_file" '
        function output(line) {
            print line
            previous_blank = (line == "")
        }
        function emit_insertion(    count, i, lines) {
            if (!previous_blank) output("")
            output("§ " target_section "." insertion_number)
            count = split(body, lines, "\n")
            for (i = 1; i <= count; i++) output(lines[i])
            output("")
        }
        BEGIN {
            while ((getline line < body_file) > 0) body = body (body == "" ? "" : "\n") line
            close(body_file)
            target_value = wanted
            sub(/^§ /, "", target_value)
            split(target_value, target_parts, /\./)
            target_section = target_parts[1]
            target_number = target_parts[2] + 0
            insertion_number = (mode == "after" ? target_number + 1 : target_number)
        }
        {
            line = $0
            is_paragraph = (line ~ /^§ [0-9]+\.[0-9]+$/)
            if (is_paragraph) {
                current_value = line
                sub(/^§ /, "", current_value)
                split(current_value, current_parts, /\./)
                section = current_parts[1]
                number = current_parts[2] + 0
                is_target = (section == target_section && number == target_number)
                if (is_target && target_found++) exit 2
                if (is_target && mode == "before") emit_insertion()
                if (pending_after && (is_paragraph || line ~ /^## /)) {
                    emit_insertion()
                    pending_after = 0
                }
                if (is_target && mode == "after") pending_after = 1
                if (section == target_section && number >= insertion_number) {
                    line = "§ " section "." (number + 1)
                }
            } else if (pending_after && line ~ /^## /) {
                emit_insertion()
                pending_after = 0
            }
            output(line)
        }
        END {
            if (pending_after) emit_insertion()
            if (target_found != 1) exit 3
        }
    ' "$file" > "$temporary_file" || plan_die "Paragraph was not found exactly once: $paragraph_id"
    mv "$temporary_file" "$file"
    trap - RETURN
}
