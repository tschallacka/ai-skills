#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_render_csv_table() {
    local columns="$1" csv="$2" csv_file csv_status=0
    local plan_csv_diag plan_csv_where
    [[ "$columns" =~ ^[1-9][0-9]*$ ]] || plan_die "Table column count must be a positive integer"
    csv_file="$(mktemp "${TMPDIR:-/tmp}/plan-table.XXXXXX")"
    # awk names the offending row in here rather than on stderr: "/dev/stderr"
    # is not reliable across awk implementations, and the row is what makes the
    # message actionable.
    plan_csv_diag="$(mktemp "${TMPDIR:-/tmp}/plan-table-diag.XXXXXX")"
    trap 'rm -f "$csv_file" "$plan_csv_diag"' RETURN
    plan_decode_escaped_newlines "$csv" > "$csv_file"
    awk -v diag="$plan_csv_diag" -v expected="$columns" '
        function parse_csv(line, fields,    i, ch, next_ch, quoted, field, count) {
            for (i = 1; i <= length(line); i++) {
                ch = substr(line, i, 1)
                if (ch == "\\" && substr(line, i + 1, 1) == "\"") {
                    field = field "\""
                    i++
                } else if (ch == "\"") {
                    next_ch = substr(line, i + 1, 1)
                    if (quoted && next_ch == "\"") {
                        field = field "\""
                        i++
                    } else {
                        quoted = !quoted
                    }
                } else if (ch == "," && !quoted) {
                    fields[++count] = field
                    field = ""
                } else {
                    field = field ch
                }
            }
            if (quoted) return -1
            fields[++count] = field
            return count
        }
        function emit_row(fields, count,    i, cleaned, p) {
            printf "|"
            for (i = 1; i <= count; i++) {
                # A literal pipe is spelled \| in the cell and emitted verbatim:
                # GFM renders \| inside a table row as a pipe. An unescaped
                # pipe would split the Markdown row, so strip the escapes and
                # whatever raw pipe remains is a fault.
                cleaned = fields[i]
                while ((p = index(cleaned, "\\|")) > 0)
                    cleaned = substr(cleaned, 1, p - 1) substr(cleaned, p + 2)
                if (index(cleaned, "|") > 0) { printf "row %d, column %d", NR, i > diag; exit 4 }
                if (fields[i] ~ /\r/) { printf "row %d, column %d", NR, i > diag; exit 7 }
                printf " %s |", fields[i]
            }
            printf "\n"
        }
        {
            if ($0 ~ /^[[:space:]]*$/) { printf "row %d", NR > diag; exit 5 }
            count = parse_csv($0, fields)
            if (count < 0) { printf "row %d", NR > diag; exit 2 }
            if (count != expected) { printf "row %d has %d", NR, count > diag; exit 3 }
            emit_row(fields, count)
            if (NR == 1) {
                printf "|"
                for (i = 1; i <= expected; i++) printf "---|"
                printf "\n"
            }
        }
        END { if (NR == 0) exit 6 }
    ' "$csv_file" || csv_status=$?
    if [ "${csv_status:-0}" -ne 0 ]; then
        plan_csv_where="$(cat "$plan_csv_diag" 2>/dev/null || true)"
        case "$csv_status" in
            2) plan_die "CSV ${plan_csv_where:-input} has an unbalanced double quote; a quoted cell needs a closing quote, and a literal quote inside one is doubled" 65 ;;
            3) plan_die "CSV ${plan_csv_where:-row has the wrong number of} columns, expected $columns comma-separated columns on every row" 65 ;;
            4) plan_die "CSV ${plan_csv_where:-input} contains an unescaped pipe character, which would break the Markdown table; spell a literal pipe as \\| in the cell, or reword" 65 ;;
            5) plan_die "CSV ${plan_csv_where:-input} is blank; remove the empty row rather than leaving a gap between records" 65 ;;
            6) plan_die "CSV input is empty; expected $columns comma-separated columns on at least one row" 65 ;;
            7) plan_die "CSV ${plan_csv_where:-input} contains a carriage return: the file has CRLF line endings. Convert it to LF" 65 ;;
            *) plan_die "CSV could not be rendered; awk exited $csv_status" 70 ;;
        esac
    fi
    rm -f "$csv_file"
    trap - RETURN
}
