#!/usr/bin/env bash
# MODE: DEV
# B22 regression — the Findings table is rewritten only after the keys minted.
#
# update-adversarial-review.sh ran mint-fix-keys.sh as its last statement,
# after the history append, the review-file rename, and the incoming-file
# removal. A gated row whose ids cannot be minted (e.g. a Work unit cell of
# W01-W18) died there — leaving the new table in place, the old rows archived,
# and fix-keys.json unwritten. The approval gate then demanded claims for keys
# that were never minted, across a role boundary.
#
# Now the whole rewrite is minted against a throwaway copy first: any minting
# refusal lands while every plan file still holds its original bytes.
#
# Controls: conforming rows complete end to end (table rewritten, incoming
# consumed, fix-keys.json written), and direct mint invocations keep refusing.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-mint-order-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

fail() { t_fail "$*"; }

header='ID,Missing or over-broad item,Required plan change,Status,Work unit'

seed_plan() { # <plan-dir>
    local plan="$1"
    mkdir -p "$plan"
    "$script_dir/create-adversarial-review.sh" "$plan" >/dev/null
}

# --- the B22 shape: a gated row that cannot mint must change nothing ---------
plan_bad="$temporary_root/bad"
seed_plan "$plan_bad"
cp "$plan_bad/adversarial-review.md" "$temporary_root/review-before.md"
printf '%s\n%s\n' "$header" 'AR-61,ranged work unit cell,split it,✅ resolved,W01-W18' \
    > "$plan_bad/adversarial-review-incoming.md"

rc=0
err="$(mktemp "$temporary_root/bad.err.XXXXXX")"
"$script_dir/update-adversarial-review.sh" "$plan_bad" >/dev/null 2>"$err" || rc=$?
t_assert_eq "the unmintable row still refuses" "$([ "$rc" -ne 0 ] && echo yes || echo no)" yes
grep -qiE 'could not be minted|non-conforming' "$err" || fail "refusal lost the minting diagnosis: $(cat "$err")"
cmp -s "$temporary_root/review-before.md" "$plan_bad/adversarial-review.md" \
    || fail "the refused call rewrote adversarial-review.md"
if [ -f "$plan_bad/adversarial-review-history.md" ]; then
    fail "the refused call archived a history cycle"
fi
[ -e "$plan_bad/adversarial-review-incoming.md" ] \
    || fail "the refused call consumed the reviewer's incoming file"
if [ -f "$plan_bad/fix-keys.json" ]; then
    fail "a failed mint wrote fix-keys.json"
fi

# --- retry after fixing the cells completes end to end -----------------------
printf '%s\n%s\n' "$header" 'AR-61,ranged work unit cell,split it,✅ resolved,W01' \
    > "$plan_bad/adversarial-review-incoming.md"
rc=0
"$script_dir/update-adversarial-review.sh" "$plan_bad" >/dev/null 2>&1 || rc=$?
t_assert_eq "repaired retry exits 0" "$rc" 0
grep -Fq '| AR-61 |' "$plan_bad/adversarial-review.md" || fail "repaired retry did not render the finding"
if [ ! -f "$plan_bad/fix-keys.json" ]; then
    fail "repaired retry minted no keys"
fi
if [ -e "$plan_bad/adversarial-review-incoming.md" ]; then
    fail "successful run left the incoming file"
fi

# --- control: direct mint invocation keeps its own refusal -------------------
plan_direct="$temporary_root/direct"
seed_plan "$plan_direct"
awk '
    /^## Verdict$/ && !inserted {
        print "| ID | Missing or over-broad item | Required plan change | Status | Work unit |"
        print "|---|---|---|---|---|"
        print "| AR-62 | ranged cell | split it | ✅ resolved | W01-W18 |"
        print ""
        inserted = 1
    }
    { print }
' "$plan_direct/adversarial-review.md" > "$plan_direct/adversarial-review.md.new"
mv "$plan_direct/adversarial-review.md.new" "$plan_direct/adversarial-review.md"
rc=0
"$script_dir/mint-fix-keys.sh" "$plan_direct" >/dev/null 2>&1 || rc=$?
t_assert_eq "direct mint still refuses a non-conforming row" "$([ "$rc" -ne 0 ] && echo yes || echo no)" yes

t_end
