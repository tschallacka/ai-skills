#!/usr/bin/env bash
# MODE: DEV
# test-add-planning-bug.sh — the writer for a plan's defect register.
#
# planning-bugs.json had five readers and no writer, so `plan-content.sh get …
# planning-bugs` returned 66 on every plan that had ever existed. The gap that
# hid it is the one pinned here: nothing asserted that anything could produce the
# document, only that a missing one was reported as missing.
#
# The register follows the bug-report skill's schema, so the last case reads it
# back with that skill's own render recipe rather than a bespoke one.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/add-planning-bug.XXXXXX")"
trap 'rm -rf "$work"' EXIT

plan="$work/root/demo"
"$scripts/create-plan.sh" "$plan" 'Demo' >/dev/null 2>&1
register="$plan/planning-bugs.json"

add() { "$scripts/add-planning-bug.sh" "$@"; }
rc_of() { local rc=0; "$scripts/add-planning-bug.sh" "$@" >/dev/null 2>&1 || rc=$?; printf '%s' "$rc"; }

# ── the document does not exist until something writes it ───────────────────
t_assert_eq 'a plan starts with no register' "$([ -f "$register" ] && printf yes || printf no)" 'no'
rc=0
"$scripts/plan-content.sh" get "$plan" planning-bugs >/dev/null 2>&1 || rc=$?
t_assert_eq 'and asking for it reports the input missing' "$rc" '66'

# ── first use creates it, and the result is readable JSON ───────────────────
# Asserted by reading the entry back, not by the exit code: an earlier version
# wrote a zero-byte file and reported success, because jq handed /dev/null as
# input never runs its filter.
add "$plan" --id PB-01 --title 'The importer drops the last row' \
    --reproduce "printf 'a,b' | bin/import" --observed '1 row, 2 expected' \
    --expected 'both rows' --found-by 'step 02' >/dev/null
t_assert_eq 'the register is created' "$([ -s "$register" ] && printf yes || printf no)" 'yes'
t_assert_eq 'and the entry is in it' \
    "$(jq -r '.bugs[0].id' "$register")" 'PB-01'
t_assert_eq 'with the schema its readers expect' \
    "$(jq -r '.skill' "$register")" 'bug-report'
t_assert_eq 'the defaults are recorded rather than left null' \
    "$(jq -r '.bugs[0] | "\(.status)/\(.severity)/\(.priority)"' "$register")" 'reported/major/normal'
t_assert_eq 'and the timestamps are set' \
    "$(jq -r '.bugs[0] | select(.created_at != null and .updated_at != null) | "set"' "$register")" 'set'

# ── text that would break a hand-rolled writer survives ────────────────────
add "$plan" --id PB-02 --title 'The regex \d fails and it says "no input"' \
    --reproduce 'grep -E "\d" f' --observed 'exit 1' --expected 'a match' \
    --severity minor --priority low --status confirmed >/dev/null
t_assert_eq 'a backslash and quotes survive the round trip' \
    "$(jq -r '.bugs[] | select(.id == "PB-02") | .title' "$register")" \
    'The regex \d fails and it says "no input"'
t_assert_eq 'the second entry is appended, not replacing the first' \
    "$(jq -r '.bugs | length' "$register")" '2'

# ── the refusals, each with its own exit code ──────────────────────────────
t_assert_eq 'a duplicate id is refused' \
    "$(rc_of "$plan" --id PB-01 --title t --reproduce r --observed o --expected e)" '73'
t_assert_eq 'a malformed id is refused' \
    "$(rc_of "$plan" --id B1 --title t --reproduce r --observed o --expected e)" '64'
t_assert_eq 'an entry with no reproduction is refused' \
    "$(rc_of "$plan" --id PB-09 --title t --observed o --expected e)" '64'
t_assert_eq 'an unknown severity is refused' \
    "$(rc_of "$plan" --id PB-09 --title t --reproduce r --observed o --expected e --severity awful)" '64'
# A writer able to record `fixed` would let an entry arrive closed with nothing
# that ever reproduced it.
t_assert_eq 'recording an already-fixed defect is refused' \
    "$(rc_of "$plan" --id PB-09 --title t --reproduce r --observed o --expected e --status fixed)" '64'
t_assert_eq 'a missing plan directory is refused' \
    "$(rc_of "$work/absent" --id PB-09 --title t --reproduce r --observed o --expected e)" '66'
t_assert_eq 'the --plan-dir spelling works too' \
    "$(rc_of --plan-dir "$plan" --id PB-03 --title t --reproduce r --observed o --expected e)" '0'

# A damaged register is not appended to: the append would silently discard
# whatever was already recorded.
cp "$register" "$work/keep.json"
printf 'not json\n' > "$register"
t_assert_eq 'a damaged register is refused rather than overwritten' \
    "$(rc_of "$plan" --id PB-04 --title t --reproduce r --observed o --expected e)" '65'
t_assert_eq 'and the damaged file is left as it was' "$(cat "$register")" 'not json'
cp "$work/keep.json" "$register"

# ── the bug-report skill's own render reads a plan register unchanged ───────
rendered="$(jq -r '
  def glyph: {reported:"💤", confirmed:"⛔", fixed:"✅"}[.status] // "❔";
  def prank: {urgent:0, high:1, normal:2, low:3, someday:4}[.priority // ""] // 5;
  def srank: {blocking:0, major:1, minor:2, cosmetic:3}[.severity // ""] // 4;
  def idkey: [(. | scan("[0-9]+") | tonumber)?, .];
  [.bugs[]] | sort_by(prank, srank, (.idkey? // (.id | idkey)))[]
  | (. | glyph) + " " + .id' "$register")"
t_assert_contains 'the skill render lists the first entry' 'PB-01' "$rendered"
t_assert_contains 'and the second' 'PB-02' "$rendered"

# ── the read-back refuses a write that produced nothing ────────────────────
# The actionable-error path (MAINTAINER.md section 4 step 3). It needs a write
# that succeeds and yet leaves the entry out, which is what happened for real:
# jq handed an empty input never runs its filter, so the register came out
# zero-length and the script still reported success. The seam is a jq on PATH
# that passes everything through except the append -- the one call carrying
# --arg now -- which it answers with nothing.
stub_dir="$work/stub-bin"
mkdir -p "$stub_dir"
real_jq="$(command -v jq)"
cat > "$stub_dir/jq" <<STUB
#!/usr/bin/env bash
for arg in "\$@"; do
    if [ "\$arg" = now ]; then
        exit 0
    fi
done
exec "$real_jq" "\$@"
STUB
chmod +x "$stub_dir/jq"

empty_plan="$work/root/empty-write"
"$scripts/create-plan.sh" "$empty_plan" 'Demo' >/dev/null 2>&1
rc=0
PATH="$stub_dir:$PATH" "$scripts/add-planning-bug.sh" "$empty_plan" --id PB-01 \
    --title t --reproduce r --observed o --expected e >"$work/stub.out" 2>"$work/stub.err" || rc=$?
t_assert_eq 'a write that produced nothing is refused, not reported as success' "$rc" '70'
t_assert_contains 'and the message names the register and the id' 'PB-01' "$(cat "$work/stub.err")"
t_assert_eq 'no success line was printed' \
    "$(grep -c 'Recorded' "$work/stub.out" || true)" '0'
# The refusal leaves the damaged register in place rather than deleting it: a
# plan snapshot was taken before the write, so it is recoverable, and a writer
# that tidied away the evidence would hide which write went wrong. The next call
# therefore refuses it as damaged, which is the 65 path already asserted above.
t_assert_eq 'the damaged register is left for inspection' \
    "$([ -f "$empty_plan/planning-bugs.json" ] && printf yes || printf no)" 'yes'
t_assert_eq 'and it is empty, which is what went wrong' \
    "$(wc -c < "$empty_plan/planning-bugs.json" | tr -d ' ')" '0'

# The seam has to be real, or the case above passes for the wrong reason: the
# same call on a clean plan, without the stub, succeeds.
control_plan="$work/root/control"
"$scripts/create-plan.sh" "$control_plan" 'Demo' >/dev/null 2>&1
rc=0
"$scripts/add-planning-bug.sh" "$control_plan" --id PB-02 \
    --title t --reproduce r --observed o --expected e >/dev/null 2>&1 || rc=$?
t_assert_eq 'the same call without the stub succeeds' "$rc" '0'

# ── the writer is registered, or an install cannot run it ──────────────────
t_assert_eq 'the script ships' \
    "$(grep -c '^planning/scripts/add-planning-bug.sh\b' "$repo_root/planning/PACKAGE-MANIFEST.tsv")" '1'
t_assert_eq 'and is in the map' \
    "$(grep -c '^planning/scripts/add-planning-bug.sh\b' "$repo_root/planning/PACKAGE-MAP.tsv")" '1'

t_end
