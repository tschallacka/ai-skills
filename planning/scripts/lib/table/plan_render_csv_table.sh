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
    awk -v diag="$plan_csv_diag" -v expected="$columns" "$(plan_render_csv_table_awk)" "$csv_file" || csv_status=$?
    if [ "${csv_status:-0}" -ne 0 ]; then
        plan_csv_where="$(cat "$plan_csv_diag" 2>/dev/null || true)"
        plan_render_csv_table_die "$csv_status" "$plan_csv_where" "$columns"
    fi
    rm -f "$csv_file"
    trap - RETURN
}
