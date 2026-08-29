#!/usr/bin/env bash
# MODE: DEV
# test-csv-table-errors.sh — plan_render_csv_table says which CSV problem it
# found, and where.
#
# The bug this pins: six distinct awk exits (unbalanced quote, wrong column
# count, pipe in a cell, carriage return, blank row, empty input) all collapsed
# into one message about columns and pipes. A CRLF file was reported as having
# the wrong number of columns, which sends the reader looking in the wrong place.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scripts_dir="$(cd "$tests_dir/../scripts" && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/csv-errors.XXXXXX")"
trap 'rm -rf "$work"' EXIT

note_fail() { printf 'FAIL: %s\n' "$1" >&2; t_record "$1"; }

# plan_die exits, so each case runs in a subshell.
render() { # <columns> <csv>
    (
        set -euo pipefail
        plan_error_count=0
        # shellcheck source=planning/scripts/plan-map-lib.sh
        source "$scripts_dir/plan-map-lib.sh"
        source "$scripts_dir/plan-inventory-lib.sh"
        source "$scripts_dir/plan-document-lib.sh"
        plan_render_csv_table "$1" "$2"
    ) 2>&1
}

expect_message() { # <label> <columns> <csv> <needle>...
    local label="$1" columns="$2" csv="$3"; shift 3
    local rc=0 out needle
    out="$(render "$columns" "$csv")" || rc=$?
    [ "$rc" -eq 65 ] || note_fail "$label: exited $rc, want 65"
    for needle in "$@"; do
        case "$out" in
            *"$needle"*) ;;
            *) note_fail "$label: message lacked '$needle' — got: $out" ;;
        esac
    done
}

expect_message 'CRLF'            5 "AR-01,item,change,open,W01$(printf '\r')" 'carriage return' 'CRLF' 'row 1'
expect_message 'column count'    5 'AR-01,item,change'            'expected 5' 'row 1 has 3'
expect_message 'raw pipe in a cell' 5 'AR-01,it|em,change,open,W01'  'unescaped pipe character' 'row 1, column 2'
expect_message 'unbalanced quote' 5 'AR-01,"item,change,open,W01' 'unbalanced double quote' 'row 1'
expect_message 'blank row'       5 'AR-01,a,b,open,W01\n\nAR-02,a,b,open,W02' 'blank' 'row 2'
expect_message 'empty input'     5 ''                             'empty'

# The escaped spelling lands the pipe in the cell (B25): reviewer prose can
# quote table syntax by writing \|, which GFM renders as a literal pipe.
rc=0
out="$(render 5 'AR-01,it\|em,change,open,W01')" || rc=$?
[ "$rc" -eq 0 ] || note_fail "an escaped pipe exited $rc, want 0"
case "$out" in
    *'| it\|em |'*) ;;
    *) note_fail "an escaped pipe did not render verbatim — got: $out" ;;
esac
# An escape must not rescue a row that also carries a raw pipe elsewhere.
expect_message 'escaped and raw pipe' 5 'AR-01,it\|em,ch|nge,open,W01' 'unescaped pipe character' 'row 1, column 3'

# Each message must be distinguishable from the others, or the reader is no
# better off than with the single message this replaced.
seen=""
for csv in "AR-01,item,change,open,W01$(printf '\r')" 'AR-01,item,change' 'AR-01,it|em,change,open,W01' \
           'AR-01,"item,change,open,W01' 'AR-01,a,b,open,W01\n\nAR-02,a,b,open,W02'; do
    out="$(render 5 "$csv" || true)"
    first="$(printf '%s\n' "$out" | head -1)"
    case "$seen" in
        *"$first"*) note_fail "two different CSV faults produced the same message: $first" ;;
    esac
    seen="$seen$first
"
done

# The success path must be untouched.
rc=0
out="$(render 3 'a,b,c')" || rc=$?
[ "$rc" -eq 0 ] || note_fail "a valid CSV exited $rc, want 0"
case "$out" in
    *'| a | b | c |'*) ;;
    *) note_fail "a valid CSV did not render its row — got: $out" ;;
esac

if [ "$(t_failures)" -ne 0 ]; then
    printf 'test-csv-table-errors: %d failure(s).\n' "$(t_failures)" >&2
    exit 1
fi
printf 'test-csv-table-errors: PASS\n'
