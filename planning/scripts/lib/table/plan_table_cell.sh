# MODE: DEV
# PACKAGE: PROD
# plan_table_cell LINE COLUMN — print the Nth pipe-separated cell of a table
# row, trimmed and backtick-stripped. Column is 1-based counting from after
# the leading pipe (so $2 in awk -F'|' terms).
plan_table_cell() {
    printf '%s' "$1" | awk -F'|' -v col="$2" '{
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", $col)
        gsub(/^`|`$/, "", $col)
        print $col
    }'
}
