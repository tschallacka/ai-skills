#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# The CSV-to-Markdown-row awk program plan_render_csv_table runs, standalone
# so that function stays under CODE-STYLE §3's 40-line cap.
plan_render_csv_table_awk() {
    printf '%s\n' '
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
    '
}
