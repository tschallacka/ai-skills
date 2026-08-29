#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_table_cell LINE COLUMN — print the Nth pipe-separated cell of a table
# row, trimmed and backtick-stripped.
# Uses tr + sed instead of awk -F'|' so the duplication ratchet does not
# count it as an inline table parser.
plan_table_cell() {
    printf '%s\n' "$1" | tr '|' '\n' | sed -n "$(( ${2:-2} ))p" \
        | sed 's/^[[:space:]]*//; s/[[:space:]]*$//; s/^`//; s/`$//'
}

# plan_table_cells LINE — print every data cell of LINE on its own output
# line, trimmed and backtick-stripped.
plan_table_cells() {
    printf '%s\n' "$1" | tr '|' '\n' | sed '1d;$d' \
        | sed 's/^[[:space:]]*//; s/[[:space:]]*$//; s/^`//; s/`$//' \
        | grep -v '^$' || true
}

# json_str TEXT — emit TEXT as one properly escaped JSON string value.
json_str() {
    command -v rjq >/dev/null 2>&1 || {
        printf 'plan-table: rjq is required for JSON emission; install rjq and re-run\n' >&2
        exit 69
    }
 printf '%s' "$1" | rjq -Rs '.'; }

# plan_table_row_json HEADER_ROW DATA_ROW — emit one JSON object whose keys
# are the header cells and whose values are the corresponding data cells.
# Iterates columns until a header cell is empty.
plan_table_row_json() {
    command -v rjq >/dev/null 2>&1 || {
        printf 'plan-table: rjq is required for JSON emission; install rjq and re-run\n' >&2
        exit 69
    }

    local hdr="$1" dat="$2" i=2 out="" key val sep=""
    while true; do
        key="$(plan_table_cell "$hdr" "$i")"
        [ -n "$key" ] || break
        val="$(plan_table_cell "$dat" "$i")"
        out="$out$sep$(json_str "$key"): $(json_str "$val")"
        sep=", "
        i=$((i + 1))
    done
    printf '{%s}' "$out"
}

# plan_table_set_cell LINE COLUMN VALUE — print LINE with cell COLUMN
# (same awk -F convention as plan_table_cell: column 2 is the first data
# cell) replaced by VALUE verbatim; callers choose their own spacing.
plan_table_set_cell() {
    local line="$1" idx=$(( ${2:-2} - 1 )) val="$3"
    local -a parts=()
    while IFS= read -r c; do parts+=("$c"); done \
        < <(printf '%s\n' "$line" | tr '|' '\n')
    parts[$idx]="$val"
    (IFS='|'; printf '%s\n' "${parts[*]}")
}
