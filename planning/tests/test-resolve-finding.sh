#!/usr/bin/env bash
# MODE: DEV
# test-resolve-finding — closing a finding sets its status, records exactly one
# claim, and refuses the three things that produced real defects.
#
# Usage: test-resolve-finding.sh
#
# Each refusal here corresponds to something that actually happened while the
# three steps were run by hand (T54): a finding cited by id in six work units
# that never became a row and so could never be minted or reported as unclaimed;
# a re-gated finding whose old claim stayed behind until verify-fix-keys reported
# it ignored rather than verified; and a session claiming keys it had minted.
#
# This file is shipped, so it holds to the shipped-runtime dependency rule in
# CODE-STYLE.md §1: bash, POSIX coreutils, awk, sed, grep, jq only. No python3.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scripts="$root/planning/scripts"

note_fail() { printf 'resolve-finding: %s\n' "$1" >&2; t_record "$1"; }
assert_eq() { [ "$1" = "$2" ] || note_fail "$3: expected '$1', got '$2'"; }
assert_rc() { # WANT CMD...
    local want="$1"; shift
    local rc=0
    "$@" >/dev/null 2>&1 || rc=$?
    [ "$rc" = "$want" ] || note_fail "$* : expected exit $want, got $rc"
}

work="$(mktemp -d "${TMPDIR:-/tmp}/resolve-finding.XXXXXX")"
trap 'rm -rf "$work"' EXIT
export PLANS_ROOT="$work"
plan="$work/p"

"$scripts/create-plan.sh" p "Resolve fixture" >/dev/null
"$scripts/add-goal.sh" "$plan" 01-a "A" "one demonstrable outcome" >/dev/null
"$scripts/add-work-unit.sh" "$plan" --id W01 --type source --file src/a.rs \
    --scope "a()" --subscope N/A --change "The one target." --depends-on -- \
    --goal 01-a --step 01-step-a >/dev/null
"$scripts/create-adversarial-review.sh" "$plan" >/dev/null 2>&1 || true
"$scripts/add-adversarial-finding.sh" "$plan" AR-02 "a gated finding" "fix it" \
    --status open --work-unit W01 >/dev/null 2>&1

status_of() { awk -F'|' -v f="$1" '$0 ~ "^\\| "f" " {gsub(/^ +| +$/,"",$5); print $5; exit}' "$plan/adversarial-review.md"; }
claims_for() { awk -v f="$1" '$0 ~ "^"f"\t" {n++} END {print n+0}' "$plan/fixes.md"; }

# 1. The ordinary path: one command sets the status and records one claim.
"$scripts/resolve-finding.sh" "$plan" AR-02 >/dev/null 2>&1
assert_eq "resolved" "$(status_of AR-02)" "status after resolving"
assert_eq "1" "$(claims_for AR-02)" "exactly one claim recorded"
"$scripts/verify-fix-keys.sh" "$plan" --claimed-by not-the-minter >/dev/null 2>&1 \
    || note_fail "verification failed after an ordinary resolve"

# 2. Resolving twice leaves one claim, not two. A superseded row is what made
#    verify-fix-keys report ignored rather than verified.
"$scripts/resolve-finding.sh" "$plan" AR-02 >/dev/null 2>&1
assert_eq "1" "$(claims_for AR-02)" "re-resolving does not duplicate the claim"

# 3. A finding with no row is refused. It used to be possible to remediate and
#    cite nine findings that no row carried, and the gate reported passed.
assert_rc 65 "$scripts/resolve-finding.sh" "$plan" AR-99

# 4. An ungated finding has no key, so claiming one is refused rather than
#    silently skipped.
"$scripts/add-adversarial-finding.sh" "$plan" AR-03 "ungated" "fix" \
    --status open >/dev/null 2>&1
assert_rc 65 "$scripts/resolve-finding.sh" "$plan" AR-03

# 5. The session that minted the keys cannot claim them. The gate refuses this
#    too, but refusing at the claim puts the error where the mistake is made.
minter="$(jq -r '.minted_by' "$plan/fix-keys.json")"
assert_rc 70 "$scripts/resolve-finding.sh" "$plan" AR-02 --claimed-by "$minter"
assert_eq "resolved" "$(status_of AR-02)" "a refused claim leaves the status alone"

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'resolve-finding: PASS\n'
