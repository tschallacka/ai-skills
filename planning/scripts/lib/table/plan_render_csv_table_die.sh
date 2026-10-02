#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# Map plan_render_csv_table's awk exit status to plan_die's message.
plan_render_csv_table_die() { # <status> <where> <columns>
    local status="$1" where="$2" columns="$3"
    case "$status" in
        2) plan_die "CSV ${where:-input} has an unbalanced double quote; a quoted cell needs a closing quote, and a literal quote inside one is doubled" 65 ;;
        3) plan_die "CSV ${where:-row has the wrong number of} columns, expected $columns comma-separated columns on every row" 65 ;;
        4) plan_die "CSV ${where:-input} contains an unescaped pipe character, which would break the Markdown table; spell a literal pipe as \\| in the cell, or reword" 65 ;;
        5) plan_die "CSV ${where:-input} is blank; remove the empty row rather than leaving a gap between records" 65 ;;
        6) plan_die "CSV input is empty; expected $columns comma-separated columns on at least one row" 65 ;;
        7) plan_die "CSV ${where:-input} contains a carriage return: the file has CRLF line endings. Convert it to LF" 65 ;;
        *) plan_die "CSV could not be rendered; awk exited $status" 70 ;;
    esac
}
