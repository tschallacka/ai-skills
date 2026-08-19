#!/usr/bin/env bash
# plan-context paging contract test.
#
# Guards the defect this suite used to hide: the gated reader served a 12-line
# `summary` slice of a 30-finding adversarial review, reported next_token:null,
# and a reviewer following the documented procedure filed a clean audit over a
# document it had mostly never seen. An upper-bound assertion ("no more than N
# records came back") passes for every degree of truncation, so it could not
# catch that. Every assertion here is therefore a lower bound or an equality:
#
#   * the two review surfaces (inventory, adversarial-review) default to a
#     whole-document view, so --max-records N really yields all N records;
#   * a page that withholds a record ALWAYS reports next_token, and a page that
#     withholds nothing always reports none;
#   * walking next_token from page one reassembles the document exactly —
#     gapless, duplicate-free, in order, and terminating;
#   * a token replayed after the document changed, or against another view, is
#     refused with exit 65 rather than resuming into shifted records;
#   * --max-records and --max-bytes still bound each PAGE, and the per-role
#     32768-byte cap still applies with ROLE_ID set.

set -uo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scripts="$repo_root/planning/scripts"
reader="$scripts/plan-context.sh"
role_cap=32768

tmp="$(mktemp -d "${TMPDIR:-/tmp}/plan-context-paging.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

fail=0
note_fail() { printf 'FAIL: %s\n' "$1" >&2; fail=1; }
note_pass() { printf 'PASS: %s\n' "$1"; }
assert_eq() {
    if [ "$2" = "$3" ]; then note_pass "$1"
    else note_fail "$1 (expected '$3', got '$2')"; fi
}
assert_ge() {
    if [ "$2" -ge "$3" ] 2>/dev/null; then note_pass "$1"
    else note_fail "$1 (expected >= $3, got '$2')"; fi
}
assert_le() {
    if [ "$2" -le "$3" ] 2>/dev/null; then note_pass "$1"
    else note_fail "$1 (expected <= $3, got '$2')"; fi
}
assert_files_identical() {
    if diff -q "$2" "$3" >/dev/null 2>&1; then note_pass "$1"
    else note_fail "$1 ($2 differs from $3)"; diff "$2" "$3" >&2 || true; fi
}

count_rows() { { grep -c "$1" "$2" || true; } | tr -d ' '; }
# `read` writes content on stdout; a truncated page appends one
# `next_token=<token>` line. Split them rather than grep -q (pipefail-grep-q).
token_of() { sed -n 's/^next_token=//p' "$1"; }
content_of() { sed '/^next_token=/d' "$1"; }

findings=30
plan="$tmp/plan"
"$scripts/create-plan.sh" "$plan" paging >/dev/null
"$scripts/add-goal.sh" "$plan" 01-page context 'Paging proof.' >/dev/null
"$scripts/create-adversarial-review.sh" "$plan" >/dev/null
review="$plan/adversarial-review.md"

# Seed rows straight into the Findings table: one CLI call per row re-mints keys
# and dominates the runtime, and this test only needs the table shape.
inserted="$tmp/review-seeded.md"
awk -v n="$findings" '
    { print }
    /^## Findings$/ { findings_section = 1 }
    findings_section && !seeded && /^\|---/ {
        for (i = 2; i <= n; i++) printf "| AR-%02d | Finding number %d | Change number %d | open | N/A |\n", i, i, i
        seeded = 1
    }
' "$review" > "$inserted"
mv "$inserted" "$review"
"$reader" init --plan-dir "$plan" >/dev/null

assert_eq 'fixture holds every finding row' "$(count_rows '^| AR-' "$review")" "$findings"

# ---- 1. the review surfaces default to a whole-document view ----------------
for document in adversarial-review inventory; do
    out="$tmp/default-$document.txt"
    "$reader" read --plan-dir "$plan" --document "$document" --read-only --max-records 100000 > "$out" 2>&1
    content_of "$out" > "$out.content"
    assert_files_identical "--document $document default view returns the whole document" \
        "$out.content" "$plan/$([ "$document" = inventory ] && printf 'work-unit-inventory.md' || printf 'adversarial-review.md')"
done

rows="$(count_rows '^| AR-' "$tmp/default-adversarial-review.txt.content")"
assert_eq "--max-records $findings returns all $findings finding rows" "$rows" "$findings"

# ---- 2. a complete page withholds nothing and reports no token --------------
assert_eq 'a complete page reports no next_token' "$(token_of "$tmp/default-adversarial-review.txt")" ''
json="$("$reader" read --plan-dir "$plan" --document adversarial-review --read-only --max-records 100000 --format json)"
case "$json" in
    *'"next_token":null'*) note_pass 'a complete page reports next_token null in json' ;;
    *) note_fail "a complete page reports next_token null in json (got ${json##*,})" ;;
esac

# ---- 3. a truncated page always reports a token -----------------------------
document_records="$(wc -l < "$tmp/default-adversarial-review.txt.content" | tr -d ' ')"
page_size=7
short="$tmp/page1.txt"
"$reader" read --plan-dir "$plan" --document adversarial-review --read-only --max-records "$page_size" > "$short"
assert_eq 'a page bounded by --max-records emits exactly that many records' \
    "$(wc -l < <(content_of "$short") | tr -d ' ')" "$page_size"
token="$(token_of "$short")"
case "$token" in
    continue:*) note_pass 'a truncated page reports a non-null next_token' ;;
    *) note_fail 'a truncated page reports a non-null next_token (got empty)' ;;
esac
json="$("$reader" read --plan-dir "$plan" --document adversarial-review --read-only --max-records "$page_size" --format json)"
case "$json" in
    *'"next_token":"continue:'*) note_pass 'a truncated page reports next_token in json' ;;
    *) note_fail 'a truncated page reports next_token in json' ;;
esac

# ---- 4. the token walk is gapless, duplicate-free, ordered, terminating -----
reassembled="$tmp/reassembled.md"
: > "$reassembled"
token=''
pages=0
page_limit=$(( document_records + 2 ))
while [ "$pages" -lt "$page_limit" ]; do
    pages=$((pages + 1))
    page="$tmp/walk-$pages.txt"
    if [ -z "$token" ]; then
        "$reader" read --plan-dir "$plan" --document adversarial-review --read-only --max-records "$page_size" > "$page"
    else
        "$reader" read --plan-dir "$plan" --document adversarial-review --read-only --max-records "$page_size" --token "$token" > "$page"
    fi
    content_of "$page" >> "$reassembled"
    token="$(token_of "$page")"
    [ -n "$token" ] || break
done
assert_le 'the token walk terminates within one page per record' "$pages" "$document_records"
assert_eq 'the token walk consumed every page' "$token" ''
assert_files_identical 'the reassembled pages equal the document exactly' "$reassembled" "$review"
assert_eq 'the reassembled pages hold every finding row exactly once' \
    "$(count_rows '^| AR-' "$reassembled")" "$findings"
assert_eq 'no finding row is duplicated across pages' \
    "$({ grep '^| AR-' "$reassembled" || true; } | sort | uniq -d | wc -l | tr -d ' ')" '0'
assert_ge 'the walk really paged rather than returning one page' "$pages" 2

# A byte-driven walk must be gapless too: the page boundary is chosen by the
# byte budget there, so a page that ends mid-record would lose the tail while
# the cursor moved past it.
byte_walk="$tmp/byte-walk.md"
: > "$byte_walk"
token=''
byte_pages=0
while [ "$byte_pages" -lt "$page_limit" ]; do
    byte_pages=$((byte_pages + 1))
    page="$tmp/byte-page-$byte_pages.txt"
    if [ -z "$token" ]; then
        "$reader" read --plan-dir "$plan" --document adversarial-review --read-only --max-bytes 300 > "$page"
    else
        "$reader" read --plan-dir "$plan" --document adversarial-review --read-only --max-bytes 300 --token "$token" > "$page"
    fi
    assert_le "byte-driven page $byte_pages stays inside --max-bytes" \
        "$(wc -c < <(content_of "$page") | tr -d ' ')" 300
    content_of "$page" >> "$byte_walk"
    token="$(token_of "$page")"
    [ -n "$token" ] || break
done
assert_eq 'the byte-driven walk consumed every page' "$token" ''
assert_files_identical 'a byte-driven walk also reassembles the document exactly' "$byte_walk" "$review"
assert_ge 'the byte-driven walk really paged' "$byte_pages" 2

# ---- 5. a token minted against changed content is refused fail-closed ------
stale_token="$(token_of "$tmp/page1.txt")"
printf '| AR-99 | Injected after the token was minted | N/A | open | N/A |\n' >> "$review"
stale_out="$tmp/stale.txt"
"$reader" read --plan-dir "$plan" --document adversarial-review --read-only \
    --max-records "$page_size" --token "$stale_token" > "$stale_out" 2>"$stale_out.err"
assert_eq 'a token replayed after the document changed is refused with 65' "$?" '65'
assert_eq 'the refused read returns no content' "$(wc -c < "$stale_out" | tr -d ' ')" '0'
case "$(cat "$stale_out.err")" in
    stale:*) note_pass 'the refusal names itself stale on stderr' ;;
    *) note_fail "the refusal names itself stale on stderr (got '$(cat "$stale_out.err")')" ;;
esac

# ---- 6. a token is bound to the view it was minted for ---------------------
fresh_token="$(token_of <("$reader" read --plan-dir "$plan" --document adversarial-review --read-only --max-records "$page_size"))"
"$reader" read --plan-dir "$plan" --document adversarial-review --read-only \
    --max-records "$page_size" --view summary --token "$fresh_token" >/dev/null 2>&1
assert_eq 'a token replayed against another view is refused with 65' "$?" '65'
"$reader" read --plan-dir "$plan" --document adversarial-review --read-only --token 'continue:nope' >/dev/null 2>&1
assert_eq 'a malformed token is a usage error, not a silent full read' "$?" '2'

# ---- 6b. the json envelope escapes what JSON requires ----------------------
# The reader's --format json path escapes backslash, double quote and newline.
# It used to do that with ${var//$'"'/...}, which bash 3.2 cannot parse at all
# (PORTABILITY.md: pattern-substitution-quote), so on the floor every json read
# died instead of returning a document. Pin the escaped forms directly rather
# than through jq, which is not guaranteed present.
printf '%s\n' '| AR-ESC | a backslash \ and a "quoted" phrase | N/A | open | N/A |' >> "$review"
escaped="$("$reader" read --plan-dir "$plan" --document adversarial-review --read-only \
    --max-records 100000 --format json)"
case "$escaped" in
    *'a backslash \\ and a \"quoted\" phrase'*)
        note_pass 'json escapes a backslash and a double quote in the content' ;;
    *) note_fail "json escapes a backslash and a double quote in the content (got ${escaped#*AR-ESC})" ;;
esac
case "$escaped" in
    *'|\n|'*|*'---|\n'*) note_pass 'json encodes record separators as \\n' ;;
    *) note_fail 'json encodes record separators as \\n' ;;
esac

# ---- 7. --max-bytes bounds each page, and the per-role cap still applies ----
wide="$tmp/wide.txt"
"$reader" read --plan-dir "$plan" --document adversarial-review --read-only --max-bytes 400 > "$wide"
assert_le '--max-bytes bounds the page content' "$(wc -c < <(content_of "$wide") | tr -d ' ')" 400
case "$(token_of "$wide")" in
    continue:*) note_pass 'a page cut short by --max-bytes reports a next_token' ;;
    *) note_fail 'a page cut short by --max-bytes reports a next_token' ;;
esac

# A single record wider than the whole budget must still advance, or paging
# could never terminate; it is reported truncated, never silently complete.
narrow="$tmp/narrow.txt"
"$reader" read --plan-dir "$plan" --document adversarial-review --read-only --max-bytes 12 > "$narrow"
assert_le 'an over-wide record is clipped to the byte budget' "$(wc -c < <(content_of "$narrow") | tr -d ' ')" 13
case "$(token_of "$narrow")" in
    continue:*:*:1) note_pass 'an over-wide record still advances the cursor' ;;
    *) note_fail "an over-wide record still advances the cursor (got '$(token_of "$narrow")')" ;;
esac

# Grow the review past the per-role cap so the cap, not the document, binds.
padding=0
while [ "$padding" -lt 600 ]; do
    padding=$((padding + 1))
    printf '| AR-P%03d | %s | %s | open | N/A |\n' "$padding" \
        'Padding row to push the document past the per-role byte cap' 'No change required' >> "$review"
done
assert_ge 'the padded review exceeds the per-role byte cap' "$(wc -c < "$review" | tr -d ' ')" "$((role_cap + 1))"
capped="$tmp/capped.txt"
ROLE_ID=chris "$reader" read --plan-dir "$plan" --document adversarial-review --read-only \
    --max-bytes 1000000 --max-records 100000 > "$capped"
assert_le 'ROLE_ID caps the page at the per-role byte budget' \
    "$(wc -c < <(content_of "$capped") | tr -d ' ')" "$role_cap"
case "$(token_of "$capped")" in
    continue:*) note_pass 'a page cut short by the per-role cap reports a next_token' ;;
    *) note_fail 'a page cut short by the per-role cap reports a next_token' ;;
esac

[ "$fail" -eq 0 ] || { printf 'plan-context paging contract: FAILED\n' >&2; exit 1; }
printf 'plan-context paging contract: PASS\n'
