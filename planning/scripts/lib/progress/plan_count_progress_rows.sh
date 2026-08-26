# plan_count_progress_rows FILE STATUS_CELL — print "completed total" for a
# progress tracker table: every data row counts, rows whose status cell
# contains "completed" count twice. Header (Goalname) and dash separator rows
# never count. Top-level by necessity, not style: bash 3.2 cannot parse a
# `case` inside $( ) or < <( ) (see PORTABILITY.md, case-in-substitution), so
# call sites keep only this function's invocation inside the substitution —
# read -r a b < <(plan_count_progress_rows "$file" 5).
plan_count_progress_rows() {
    [ -n "${1:-}" ] || { printf 'plan_count_progress_rows: file required\n' >&2; return 1; }
    [ -f "$1" ] || { printf 'plan_count_progress_rows: no such file: %s\n' "$1" >&2; return 1; }
    local cfile="$1" status_cell="${2:-5}"
    local completed=0 total=0 prow pgoal pstatus
    while IFS= read -r prow || [ -n "$prow" ]; do
        case "$prow" in '|'*) ;; *) continue ;; esac
        pgoal="$(plan_table_cell "$prow" 2)"
        pstatus="$(plan_table_cell "$prow" "$status_cell")"
        case "$pgoal" in Goalname) continue ;; esac
        [[ $pgoal =~ ^-+$ ]] && continue
        [[ $pstatus =~ ^-+$ ]] && continue
        total=$((total + 1))
        case "$pstatus" in *completed*) completed=$((completed + 1)) ;; esac
    done < "$cfile"
    printf '%s %s\n' "$completed" "$total"
}
