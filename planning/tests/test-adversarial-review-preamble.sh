#!/usr/bin/env bash
# MODE: DEV
# B19 regression — update-adversarial-review.sh must read a reviewer's findings
# document, not just bare CSV rows.
#
# adversarial-review-incoming.md is written per protocol as a small document:
# a title line, sometimes blank lines, usually the reviewer's own copy of the
# header row, then the finding rows. Every non-empty line used to be handed to
# plan_render_csv_table, so a prose line was parsed as a row and refused with a
# column-count error naming the wrong line. Title/comment lines, blank lines,
# and one repeated header row are now skipped before rendering.
#
# The control keeps bare-CSV sources working: rows with no preamble at all
# (the stdin/--file shape) still render unchanged.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-review-preamble-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

fail() { t_fail "$*"; }

header='ID,Missing or over-broad item,Required plan change,Status,Work unit'

preamble_doc="# Adversarial review incoming findings — Chris

$header
AR-41,title mentions W02,add the missing edge,✅ resolved,W01
AR-42,next finding,tighten scope,💤 open,N/A

AR-43,last finding,same,✅ resolved,W01
"

seed_plan() { # <plan-dir>
    local plan="$1"
    mkdir -p "$plan"
    "$script_dir/create-adversarial-review.sh" "$plan" >/dev/null
}

count_rows() { # <file> <needle>
    grep -cF "| $2 |" "$1" || true
}

# --- incoming-file mode tolerates the documented preamble --------------------
plan_incoming="$temporary_root/incoming"
seed_plan "$plan_incoming"
printf '%s\n' "$preamble_doc" > "$plan_incoming/adversarial-review-incoming.md"
rc=0
"$script_dir/update-adversarial-review.sh" "$plan_incoming" >/dev/null 2>&1 || rc=$?
t_assert_eq "incoming run with preamble exits 0" "$rc" 0
[ "$rc" -eq 0 ] || {
    t_end
    exit 0
}
review="$plan_incoming/adversarial-review.md"
t_assert_eq "row AR-41 rendered" "$(count_rows "$review" AR-41)" 1
t_assert_eq "row AR-42 rendered" "$(count_rows "$review" AR-42)" 1
t_assert_eq "row AR-43 after a blank line rendered" "$(count_rows "$review" AR-43)" 1
t_assert_eq "title line never reached the table" \
    "$(grep -cF 'Adversarial review incoming findings' "$review" || true)" 0
t_assert_eq "exactly one header row in Findings" \
    "$(awk '/^## Findings$/{f=1;next} /^## Verdict$/{f=0} f && /^\| ID \|/{n++} END{print n+0}' "$review")" 1

# --- --file mode gets the same tolerance -------------------------------------
plan_filemode="$temporary_root/filemode"
seed_plan "$plan_filemode"
printf '%s\n' "$preamble_doc" > "$temporary_root/preamble.csv"
rc=0
"$script_dir/update-adversarial-review.sh" "$plan_filemode" --file "$temporary_root/preamble.csv" >/dev/null 2>&1 || rc=$?
t_assert_eq "--file run with preamble exits 0" "$rc" 0
t_assert_eq "--file row AR-41 rendered" "$(count_rows "$plan_filemode/adversarial-review.md" AR-41)" 1

# --- a preamble-only document is refused with its own message ----------------
plan_empty="$temporary_root/prelude-only"
seed_plan "$plan_empty"
{
    printf '# Adversarial review incoming findings — Chris\n\n'
    printf '%s\n' "$header"
} > "$plan_empty/adversarial-review-incoming.md"
out="$(mktemp "$temporary_root/refused.XXXXXX")"
rc=0
"$script_dir/update-adversarial-review.sh" "$plan_empty" >/dev/null 2>"$out" || rc=$?
t_assert_eq "preamble-only input refuses nonzero" "$([ "$rc" -ne 0 ] && echo yes || echo no)" yes
grep -q 'no finding rows' "$out" || fail "refusal does not name the missing rows: $(cat "$out")"

# --- control: bare rows still work through stdin ------------------------------
plan_bare="$temporary_root/bare"
seed_plan "$plan_bare"
rc=0
printf '%s\n%s\n' "$header" 'AR-51,bare rows,change it,✅ resolved,N/A' \
    | "$script_dir/update-adversarial-review.sh" "$plan_bare" >/dev/null 2>&1 || rc=$?
t_assert_eq "bare CSV via stdin exits 0" "$rc" 0
t_assert_eq "bare row AR-51 rendered" "$(count_rows "$plan_bare/adversarial-review.md" AR-51)" 1

# --- --check validates without writing (T8) -----------------------------------
# A reviewer should learn about a malformed row while still in the conversation,
# not when the coordinator lands the findings after they left. --check runs the
# same shape gate and the same mint preview as a write, then stops: no table
# rewrite, no history archive, no keys, and the incoming file stays for the real
# run.
plan_check="$temporary_root/checkmode"
seed_plan "$plan_check"
printf '%s\n' "$preamble_doc" > "$plan_check/adversarial-review-incoming.md"
review_before="$(cat "$plan_check/adversarial-review.md")"
check_out="$temporary_root/check-out"
rc=0
"$script_dir/update-adversarial-review.sh" --check "$plan_check" > "$check_out" 2>&1 || rc=$?
t_assert_eq "--check on valid rows exits 0" "$rc" 0
grep -Fq 'nothing was written' "$check_out" || fail "--check does not say nothing was written: $(cat "$check_out")"
t_assert_eq "--check left the Findings table untouched" "$(cat "$plan_check/adversarial-review.md")" "$review_before"
[ -f "$plan_check/adversarial-review-incoming.md" ] || fail "--check consumed the incoming file"

printf 'AR-01,only,three\n' > "$plan_check/adversarial-review-incoming.md"
rc=0
"$script_dir/update-adversarial-review.sh" --check "$plan_check" >/dev/null 2>&1 || rc=$?
t_assert_eq "--check refuses a malformed row with exit 65" "$rc" 65

t_end
