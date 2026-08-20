#!/usr/bin/env bash
# test-lib-table.sh — the table functions, each sourced on its own.
#
# The unit layer. The CSV renderer has six distinct refusals and one of them --
# treating row 1 as the header -- is correct for its --table-paragraph caller and
# was wrong for the review writer, which passed findings straight in and lost the
# first one of every cycle. Pinning the contract here says which caller is at
# fault next time.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
lib="$repo_root/planning/scripts/lib"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/lib-table.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The renderer needs the decoder, the fatal path and the temp registry.
renderer='table/plan_render_csv_table.sh core/plan_die.sh core/plan_decode_escaped_newlines.sh core/00-state.sh'

unit() { # <group/file>... -- <expression>
    local files=() f prelude=''
    while [ "$#" -gt 0 ] && [ "$1" != '--' ]; do files+=("$1"); shift; done
    shift
    for f in "${files[@]}"; do prelude="$prelude source '$lib/$f';"; done
    "$BASH" -c "set -uo pipefail; $prelude $1" 2>&1
}
unit_rc() { # <group/file>... -- <expression>
    local files=() f prelude='' rc=0
    while [ "$#" -gt 0 ] && [ "$1" != '--' ]; do files+=("$1"); shift; done
    shift
    for f in "${files[@]}"; do prelude="$prelude source '$lib/$f';"; done
    "$BASH" -c "set -uo pipefail; $prelude $1" >/dev/null 2>&1 || rc=$?
    printf '%s' "$rc"
}

# ── plan_render_csv_table: row 1 is the header, by contract ─────────────────
# The delimiter follows row 1. That is why a caller passing data rows loses the
# first one, and why update-adversarial-review.sh must prepend a header.
# shellcheck disable=SC2086
rendered="$(unit $renderer -- 'plan_render_csv_table 2 "Head A,Head B\nvalue 1,value 2"')"
t_assert_eq 'the header row is emitted first' \
    "$(printf '%s\n' "$rendered" | sed -n '1p')" '| Head A | Head B |'
t_assert_eq 'the delimiter follows row 1, not the data' \
    "$(printf '%s\n' "$rendered" | sed -n '2p')" '|---|---|'
t_assert_eq 'the data row follows the delimiter' \
    "$(printf '%s\n' "$rendered" | sed -n '3p')" '| value 1 | value 2 |'

# ── its refusals, each with its own exit code and a named row ──────────────
# shellcheck disable=SC2086
t_assert_eq 'a row with the wrong column count is refused' \
    "$(unit_rc $renderer -- 'plan_render_csv_table 3 "a,b,c\nd,e"')" '65'
t_assert_contains 'and the message names the row' 'row 2' \
    "$(unit $renderer -- 'plan_render_csv_table 3 "a,b,c\nd,e" || true')"
t_assert_eq 'a pipe in a cell is refused' \
    "$(unit_rc $renderer -- 'plan_render_csv_table 2 "a,b\nc,d|e"')" '65'
t_assert_eq 'an unbalanced quote is refused' \
    "$(unit_rc $renderer -- 'plan_render_csv_table 2 "a,b\n\"unclosed,d"')" '65'
t_assert_eq 'a blank row is refused' \
    "$(unit_rc $renderer -- 'plan_render_csv_table 2 "a,b\n\nc,d"')" '65'
t_assert_eq 'empty input is refused' \
    "$(unit_rc $renderer -- 'plan_render_csv_table 2 ""')" '65'
t_assert_eq 'a non-numeric column count is refused' \
    "$(unit_rc $renderer -- 'plan_render_csv_table two "a,b"')" '64'
# A quoted cell may hold a comma, which is the reason for a CSV parser at all.
t_assert_contains 'a quoted comma stays inside one cell' '| a, b |' \
    "$(unit $renderer -- 'plan_render_csv_table 2 "head,other\n\"a, b\",second"')"

# ── plan_review_gated_pairs: only conforming ids, header excluded ───────────
printf '# Review\n\n## Findings\n\n' > "$work/review.md"
printf '| ID | Item | Change | Status | Work unit |\n|---|---|---|---|---|\n' >> "$work/review.md"
printf '| AR-01 | a | b | done | W05 |\n' >> "$work/review.md"
printf '| AR-02 | c | d | done | N/A |\n' >> "$work/review.md"
printf '| AR-03 | e | f | done | W07 |\n\n## Verdict\n' >> "$work/review.md"
pairs="$(unit table/plan_review_gated_pairs.sh -- "plan_review_gated_pairs '$work/review.md'")"
t_assert_eq 'two gated pairs, the N/A row skipped' "$(printf '%s\n' "$pairs" | grep -c .)" '2'
t_assert_contains 'the first pair is present' 'AR-01' "$pairs"
t_assert_contains 'the third pair is present' 'AR-03' "$pairs"
t_assert_eq 'the header row is not a pair' "$(printf '%s\n' "$pairs" | grep -c '^ID')" '0'
# Everything after the Verdict heading is out of scope, so a later table cannot
# smuggle a pair in.
printf '| AR-99 | later | table | done | W99 |\n' >> "$work/review.md"
t_assert_eq 'a row after the Verdict heading is ignored' \
    "$(unit table/plan_review_gated_pairs.sh -- "plan_review_gated_pairs '$work/review.md'" | grep -c 'AR-99')" '0'
t_assert_eq 'a missing review file yields nothing and succeeds' \
    "$(unit_rc table/plan_review_gated_pairs.sh -- "plan_review_gated_pairs '$work/absent.md'")" '0'

# ── plan_testing_requirement_row: the row for one goal ─────────────────────
printf '# Goal\n\n## Testing requirement\n\n' > "$work/goal.md"
printf '| Requires testing | Reason | Proof |\n|---|---|---|\n| yes | it renders | a browser check |\n' >> "$work/goal.md"
t_assert_contains 'the requirement row is found' 'yes' \
    "$(unit table/plan_testing_requirement_row.sh -- "plan_testing_requirement_row '$work/goal.md'")"

t_end
