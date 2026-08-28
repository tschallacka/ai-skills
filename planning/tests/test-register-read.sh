#!/usr/bin/env bash
# MODE: DEV
# test-register-read — the register read side answers the questions callers
# were hand-rolling, including the two that got wrong answers.
#
# Usage: test-register-read.sh
#
# Both failures this pins actually happened (T60): a caller read the wrong
# array and reported a register as nearly empty while an open task sat in the
# other one, and a hand-rolled next-id crashed on the existing id T1e because
# it assumed every id parses as a number.
#
# This file is shipped, so it holds to the shipped-runtime dependency rule in
# CODE-STYLE.md §1: bash, POSIX coreutils, awk, sed, grep, jq only. No python3.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
reader="$root/planning/scripts/register-read.sh"

note_fail() { printf 'register-read: %s\n' "$1" >&2; t_record "$1"; }
assert_eq() { [ "$1" = "$2" ] || note_fail "$3: expected '$1', got '$2'"; }

work="$(mktemp -d "${TMPDIR:-/tmp}/register-read.XXXXXX")"
trap 'rm -rf "$work"' EXIT

todo="$work/TODO.json"
cat > "$todo" <<'JSON'
{
  "comment": "fixture",
  "skill": "todo",
  "skill_version": "1.4.2",
  "tasks": [
    {"id": "T1e", "title": "A non-numeric id", "status": "decided", "priority": "low"},
    {"id": "T7",  "title": "An open one", "status": "open", "priority": "high",
     "surfaces": ["planning/scripts/thing.sh"], "updated_at": "2026-08-28T10:00:00Z"},
    {"id": "T9",  "title": "A done one", "status": "done", "priority": "normal",
     "updated_at": "2026-01-01T00:00:00Z"}
  ]
}
JSON

# 1. next-id survives an id that is not a number. A hand-rolled max over the
#    numeric suffixes crashed here; the shared helper has always skipped it.
assert_eq "10" "$("$reader" todo next-id --file "$todo")" "next-id past a non-numeric id"

# 2. Filters select, and an absent filter does not match the empty string.
assert_eq "1" "$("$reader" todo count --status open --file "$todo")" "count by status"
assert_eq "3" "$("$reader" todo count --file "$todo")" "count with no filter is everything"
assert_eq "1" "$("$reader" todo count --priority high --file "$todo")" "count by priority"

# 3. Surface search matches inside the array, not against its rendering.
assert_eq "1" "$("$reader" todo count --surface thing.sh --file "$todo")" "count by surface"
assert_eq "0" "$("$reader" todo count --surface nowhere.sh --file "$todo")" "surface that matches nothing"

# 4. show prints the entry and fails on an id that is absent, so a caller can
#    branch on the exit status instead of parsing the output for emptiness.
"$reader" todo show T7 --file "$todo" >/dev/null 2>&1 \
    || note_fail "show did not find an id that is present"
if "$reader" todo show T404 --file "$todo" >/dev/null 2>&1; then
    note_fail "show succeeded for an id that is absent"
fi

# 5. report counts the live entries and --since narrows by update time.
case "$("$reader" todo report --file "$todo" | head -1)" in
    "1 open of 3 total") ;;
    *) note_fail "report header: got '$("$reader" todo report --file "$todo" | head -1)'" ;;
esac
lines="$("$reader" todo report --since 2026-06-01 --file "$todo" | grep -c '^T' || true)"
assert_eq "1" "$lines" "report --since keeps the recently updated entry only"
lines="$("$reader" todo report --since 2027-01-01 --file "$todo" | grep -c '^T' || true)"
assert_eq "0" "$lines" "report --since in the future keeps nothing"

# 6. A missing register is named, not treated as empty: exit 66, not 0 rows.
if "$reader" todo count --file "$work/absent.json" >/dev/null 2>&1; then
    note_fail "a missing register was read as an empty one"
fi

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'register-read: PASS\n'
