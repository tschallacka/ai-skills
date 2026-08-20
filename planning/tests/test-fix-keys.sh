#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"

# test-fix-keys.sh — reviewer-gated fix keys: mint (W01/W12), verify (W02),
# failure matrix (W04), schema conformance of the 5-column findings table
# (W14), and the approval hook (W07) exercised through the real scripts.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-fix-keys-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

export TMPDIR="${TMPDIR:-/tmp}"
t_begin

# The shared reporter. Its own copy exited on the first finding, so one broken
# thing hid the other 92 assertions -- and this is the only test of the fix-key
# gate: minting, forged and stale keys, self-certification, the warning count.
# The prefix changes from "test-fix-keys.sh:" to the library's "FAIL:"; nothing
# outside this file read the old one, and run-tests.sh keys on the exit code.
fail() { t_fail "$*"; }

FIXED_SECRET='00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff'

seed_secret_session() {
    local sid="$1" dir="$TMPDIR/planning-agent/review-fix-keys/$1"
    mkdir -p "$dir"
    printf '%s\n' "$FIXED_SECRET" > "$dir/secret"
    chmod 700 "$dir"
    chmod 600 "$dir/secret"
}

expected_key() {
    local sid="$1" fid="$2" wu="$3"
    printf '%s' "$sid|$fid|$wu" | openssl dgst -sha256 -hmac "$FIXED_SECRET" -binary \
        | od -An -vtx1 | tr -d ' \n'
}

seed_gated_plan() {
    local plan="$1" sid="$2"
    mkdir -p "$plan"
    printf '# Plan: fixture\n\n- Status: \u60f0\u60f1\n' > /dev/null
    printf '# Plan: fixture\n\n- Status: `💤 pending`\n' > "$plan/plan-description.md"
    {
        printf '# Adversarial review: fixture\n\n'
        printf '## Review scope\n\n'
        printf '## Findings\n\n'
        printf '| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n'
        printf '|---|---|---|---|---|\n'
        printf '| AR-01 | First gap. | Implement W05. | ✅ resolved | W05 |\n'
        printf '| AR-02 | Second gap. | None. | ✅ resolved | N/A |\n'
        printf '| AR-03 | Third gap. | Implement W07. | ✅ resolved | W07 |\n\n'
        printf 'No additional substantive finding remains.\n\n'
        printf '## Verdict\n\n'
        printf '%s\n' '- Status: `💤 pending`'
    } > "$plan/adversarial-review.md"
    printf '{"session_id": "%s", "keys": {}}\n' "$sid" > "$plan/fix-keys.json"
}

write_claims() {
    local plan="$1"; shift
    : > "$plan/fixes.md"
    for pair in "$@"; do
        printf '%s\n' "$pair" >> "$plan/fixes.md"
    done
}

# --- W03: mint round trip (deterministic, secret never enters the plan) ---
plan_a="$temporary_root/plan-a"
seed_gated_plan "$plan_a" test-session-a
seed_secret_session test-session-a
"$script_dir/mint-fix-keys.sh" "$plan_a" >/dev/null 2>&1
grep -Fq '"session_id": "test-session-a"' "$plan_a/fix-keys.json" \
    || fail 'mint did not record the session id'
grep -Fq "\"AR-01\"" "$plan_a/fix-keys.json" || fail 'mint omitted gated finding AR-01'
grep -Fq "\"W05\"" "$plan_a/fix-keys.json" || fail 'mint omitted work unit W05'
grep -Fq "\"AR-03\"" "$plan_a/fix-keys.json" || fail 'mint omitted gated finding AR-03'
grep -Fq "\"W07\"" "$plan_a/fix-keys.json" || fail 'mint omitted work unit W07'
grep -Fq 'AR-02' "$plan_a/fix-keys.json" && fail 'mint gated a no-unit finding (AR-02)'
grep -Fq '00112233' "$plan_a/fix-keys.json" && fail 'the session secret leaked into the plan'
key_01="$(expected_key test-session-a AR-01 W05)"
key_03="$(expected_key test-session-a AR-03 W07)"
grep -Fq "\"W05\": \"$key_01\"" "$plan_a/fix-keys.json" \
    || fail 'derived key for AR-01/W05 does not match HMAC-SHA256(secret, sid|fid|wu)'
grep -Fq "\"W07\": \"$key_03\"" "$plan_a/fix-keys.json" \
    || fail 'derived key for AR-03/W07 does not match HMAC-SHA256(secret, sid|fid|wu)'
cp "$plan_a/fix-keys.json" "$temporary_root/fix-keys-before.json"
"$script_dir/mint-fix-keys.sh" "$plan_a" >/dev/null 2>&1
cmp -s "$temporary_root/fix-keys-before.json" "$plan_a/fix-keys.json" \
    || fail 'mint is not deterministic within a session'
rm -rf "$TMPDIR/planning-agent/review-fix-keys/test-session-a"
"$script_dir/mint-fix-keys.sh" "$plan_a" >/dev/null 2>&1 \
    || fail 're-mint after invalidation must start a fresh session (rotation)'
grep -Fq '"session_id": "test-session-a"' "$plan_a/fix-keys.json" \
    && fail 're-mint reused the invalidated session id'
grep -Fq "\"W05\": \"$key_01\"" "$plan_a/fix-keys.json" \
    && fail 're-mint derived keys from the invalidated session secret'

# --- report 3 §4.2: non-conforming finding ids fail loudly instead of silently
#     disabling the gate; minted_by identity is recorded; verify warns when the
#     claiming session is the minting session (self-certification). ---
plan_bad="$temporary_root/plan-bad"
seed_gated_plan "$plan_bad" test-session-bad
t_sed_i 's/| AR-01 | First gap. | Implement W05. | ✅ resolved | W05 |/| AR6-01 | First gap. | Implement W05. | ✅ resolved | W05 |/' \
    "$plan_bad/adversarial-review.md"
seed_secret_session test-session-bad
if "$script_dir/mint-fix-keys.sh" "$plan_bad" >"$temporary_root/mint-bad.log" 2>&1; then
    fail 'mint accepted a non-conforming finding id (AR6-01) instead of failing loudly'
fi
grep -Fq 'non-conforming id' "$temporary_root/mint-bad.log" \
    || fail 'mint did not warn about the non-conforming row'
grep -Fq 'silently disabled' "$temporary_root/mint-bad.log" \
    || fail 'mint did not explain the silent-disable risk'

# minted_by defaults to the session id; verify warns on a same-session claim.
plan_by="$temporary_root/plan-by"
seed_gated_plan "$plan_by" test-session-by
seed_secret_session test-session-by
MINTED_BY=reviewer-session-1 "$script_dir/mint-fix-keys.sh" "$plan_by" >/dev/null 2>&1 \
    || fail 'mint rejected a valid gated plan'
grep -Fq '"minted_by": "reviewer-session-1"' "$plan_by/fix-keys.json" \
    || fail 'mint did not record the minted_by identity'
key_by1="$(expected_key test-session-by AR-01 W05)"
key_by3="$(expected_key test-session-by AR-03 W07)"
write_claims "$plan_by" "AR-01	W05	$key_by1" "AR-03	W07	$key_by3"
if "$script_dir/verify-fix-keys.sh" "$plan_by" --claimed-by reviewer-session-1 \
    >"$temporary_root/verify-self.log" 2>&1; then
    fail 'verify accepted a self-certified claim set (minted and claimed by one session)'
fi
grep -Fq 'self-certification' "$temporary_root/verify-self.log" \
    || fail 'verify did not report the same-session claim as self-certification'
grep -Fq 'fix-keys verification failed' "$temporary_root/verify-self.log" \
    || fail 'self-certification did not count towards the verification failure'
"$script_dir/verify-fix-keys.sh" "$plan_by" --claimed-by fixer-session-9 \
    >"$temporary_root/verify-distinct.log" 2>&1 \
    || fail 'verify rejected a distinct-session claim'

# --- a warning has to reach the summary count. A worker found the older build
# emitting the self-certification warning after "$warnings" was read, so the
# summary said "0 warning(s)" and a caller keying on that line read a
# self-certified run as clean. Self-certification is a failure here, not a
# warning, but the summary count itself was asserted nowhere. ---
plan_warn="$temporary_root/plan-warn"
seed_gated_plan "$plan_warn" test-session-warn
seed_secret_session test-session-warn
MINTED_BY=reviewer-session-2 "$script_dir/mint-fix-keys.sh" "$plan_warn" >/dev/null 2>&1 \
    || fail 'mint rejected a valid gated plan for the warning-count case'
key_warn1="$(expected_key test-session-warn AR-01 W05)"
key_warn3="$(expected_key test-session-warn AR-03 W07)"
# One claim for a pair fix-keys.json does not gate: reported, not fatal.
write_claims "$plan_warn" "AR-01	W05	$key_warn1" "AR-03	W07	$key_warn3" \
    "AR-99	W42	deadbeef"
"$script_dir/verify-fix-keys.sh" "$plan_warn" --claimed-by fixer-session-7 \
    >"$temporary_root/verify-warn.log" 2>&1 \
    || fail 'verify failed on a claim for a pair that is merely not gated'
grep -Fq 'ignoring claim for pair AR-99/W42' "$temporary_root/verify-warn.log" \
    || fail 'verify did not report the ungated claim'
grep -Fq '1 warning(s)' "$temporary_root/verify-warn.log" \
    || fail 'the summary did not count the warning it just reported'

grep -Fq 'self-certification' "$temporary_root/verify-distinct.log" \
    && fail 'verify warned about a distinct-session claim'
"$script_dir/verify-fix-keys.sh" "$plan_by" >"$temporary_root/verify-noclaimant.log" 2>&1 \
    || fail 'verify rejected a claim with no --claimed-by'

# --- W04: verify failure matrix ---
plan_b="$temporary_root/plan-b"
seed_gated_plan "$plan_b" test-session-b
seed_secret_session test-session-b
"$script_dir/mint-fix-keys.sh" "$plan_b" >/dev/null 2>&1
key_b1="$(expected_key test-session-b AR-01 W05)"
key_b3="$(expected_key test-session-b AR-03 W07)"
if "$script_dir/verify-fix-keys.sh" "$plan_b" >/dev/null 2>&1; then
    fail 'verify passed with no fixes.md'
fi
: > "$plan_b/fixes.md"
if "$script_dir/verify-fix-keys.sh" "$plan_b" >/dev/null 2>&1; then
    fail 'verify passed with an empty fixes.md'
fi
write_claims "$plan_b" "AR-01	W05	$key_b1" "AR-03	W07	badkey"
if "$script_dir/verify-fix-keys.sh" "$plan_b" 2>"$temporary_root/verify-forged.log"; then
    fail 'verify passed with a forged claim'
fi
grep -Fq 'fix key mismatch' "$temporary_root/verify-forged.log" || fail 'forged claim not reported'
write_claims "$plan_b" "AR-01	W05	$key_b1"
if "$script_dir/verify-fix-keys.sh" "$plan_b" 2>"$temporary_root/verify-missing.log"; then
    fail 'verify passed with a missing claim for a gated pair'
fi
grep -Fq 'no fix key claim recorded for gated pair AR-03/W07' "$temporary_root/verify-missing.log" \
    || fail 'missing gated claim not reported'
write_claims "$plan_b" "AR-01	W05	$key_b1" "AR-03	W07	$key_b3" "AR-99	W99	$key_b1"
"$script_dir/verify-fix-keys.sh" "$plan_b" 2>"$temporary_root/verify-unknown.log" \
    || fail 'verify failed on an unknown well-formed pair'
grep -Fq 'ignoring claim for pair AR-99/W99' "$temporary_root/verify-unknown.log" \
    || fail 'unknown pair claim was not warned about'
printf 'AR-01\tW05\n' > "$plan_b/fixes.md"
if "$script_dir/verify-fix-keys.sh" "$plan_b" 2>"$temporary_root/verify-malformed.log"; then
    fail 'verify passed with a malformed fixes.md line'
fi
grep -Fq 'malformed fixes.md claim' "$temporary_root/verify-malformed.log" \
    || fail 'malformed claim not reported'
write_claims "$plan_b" "AR-01	W05	$key_b1" "AR-03	W07	$key_b3"
"$script_dir/verify-fix-keys.sh" "$plan_b" >/dev/null 2>&1 \
    || fail 'verify rejected matching claims'
rm -rf "$TMPDIR/planning-agent/review-fix-keys/test-session-b"
if "$script_dir/verify-fix-keys.sh" "$plan_b" 2>"$temporary_root/verify-stale.log"; then
    fail 'verify passed with an invalidated (stale) session secret'
fi
grep -Fq 'session secret missing' "$temporary_root/verify-stale.log" || fail 'stale session not reported'
rm -f "$plan_b/fix-keys.json"
"$script_dir/verify-fix-keys.sh" "$plan_b" >/dev/null 2>&1 \
    || fail 'verify rejected an ungated plan (no fix-keys.json)'

# --- W14: 5-column schema conformance ---
plan_c="$temporary_root/plan-c"
mkdir -p "$plan_c"
"$script_dir/create-adversarial-review.sh" "$plan_c" >/dev/null
t_sed_insert_before '^## Verdict$' 'No additional substantive finding remains.' \
    "$plan_c/adversarial-review.md"
grep -Fqx '| ID | Missing or over-broad item | Required plan change | Status | Work unit |' \
    "$plan_c/adversarial-review.md" || fail 'create-adversarial-review.sh did not emit the 5-column header'
grep -Fqx '|---|---|---|---|---|' "$plan_c/adversarial-review.md" \
    || fail 'create-adversarial-review.sh did not emit the 5-column separator'
grep -Fqx '| AR-01 | No finding recorded yet. | N/A | ✅ resolved | N/A |' \
    "$plan_c/adversarial-review.md" || fail 'create-adversarial-review.sh seed row is not 5-column'
grep -Fxq '|---|---|---|---|' "$plan_c/adversarial-review.md" \
    && fail 'a 4-column separator remains in adversarial-review.md'
grep -Fxq '| ID | Missing or over-broad item | Required plan change | Status |' \
    "$plan_c/adversarial-review.md" \
    && fail 'a 4-column header remains in adversarial-review.md'
"$script_dir/add-adversarial-finding.sh" "$plan_c" AR-02 'Second gap.' 'Do it.' resolved >/dev/null
grep -Fqx '| AR-02 | Second gap. | Do it. | ✅ resolved | N/A |' "$plan_c/adversarial-review.md" \
    || fail 'add-adversarial-finding.sh did not append a 5-column row'
"$script_dir/plan-mutate.sh" add-finding "$plan_c" AR-03 'Third gap.' 'Do it too.' resolved >/dev/null
grep -Fqx '| AR-03 | Third gap. | Do it too. | ✅ resolved | N/A |' "$plan_c/adversarial-review.md" \
    || fail 'plan-mutate.sh add-finding did not append a 5-column row'
printf 'ID,Missing or over-broad item,Required plan change,Status,Work unit\nAR-10,"x","y","✅ resolved",W01\n' | \
    "$script_dir/update-adversarial-review.sh" "$plan_c" >/dev/null 2>&1 \
    || fail 'update-adversarial-review.sh rejected a 5-column CSV'
grep -Fqx '| AR-10 | x | y | ✅ resolved | W01 |' "$plan_c/adversarial-review.md" \
    || fail '5-column CSV row did not render as a 5-column row'
if printf 'ID,Missing,Required,Status\nAR-11,a,b,✅ resolved\n' | \
    "$script_dir/update-adversarial-review.sh" "$plan_c" >/dev/null 2>&1; then
    fail 'update-adversarial-review.sh accepted a 4-column CSV'
fi
grep -Fq 'plan_render_csv_table 5' "$script_dir/update-adversarial-review.sh" \
    || fail 'update-adversarial-review.sh does not call plan_render_csv_table 5'
grep -Fq 'plan_render_csv_table 4' "$script_dir/update-adversarial-review.sh" \
    && fail 'update-adversarial-review.sh still calls plan_render_csv_table 4'
grep -Fq 'Required plan change, Status, Work unit' "$script_dir/update-adversarial-review.sh" \
    || fail 'update-adversarial-review.sh strings do not list the 5-column format'
grep -Fq 'Required plan change, Status)' "$script_dir/update-adversarial-review.sh" \
    && fail 'a 4-column column list remains in update-adversarial-review.sh'
grep -Fqi 'optional' "$script_dir/update-adversarial-review.sh" \
    && fail 'update-adversarial-review.sh still describes the work unit column as optional'

plan_c2="$temporary_root/plan-c2"
seed_gated_plan "$plan_c2" test-session-c
rm -f "$plan_c2/fix-keys.json"   # validator-regression fixture stays ungated
: > "$plan_c2/work-unit-inventory.md"
"$script_dir/update-plan-content.sh" --review-status "$plan_c2" approved >/dev/null 2>&1
printf '| AR-09 | Open gap. | Fix it. | 💤 open | W01 |\n' >> "$plan_c2/adversarial-review.md"
if "$script_dir/validate-plan.sh" "$plan_c2" >"$temporary_root/validate-open.log" 2>&1; then
    fail 'validate-plan.sh passed with an open 5-column finding row'
fi
grep -Fq 'Adversarial review has unresolved findings' "$temporary_root/validate-open.log" \
    || fail 'validate-plan.sh no longer detects open findings on 5-column rows'
if "$script_dir/update-plan-content.sh" --review-status "$plan_c2" approved \
    >"$temporary_root/reapprove-open.log" 2>&1; then
    fail 'approval accepted a review with an open 5-column finding row'
fi
t_sed_i 's/| AR-09 | Open gap. | Fix it. | 💤 open | W01 |/| AR-09 | Open gap. | Fix it. | ✅ resolved | W01 |/' \
    "$plan_c2/adversarial-review.md"
if "$script_dir/validate-plan.sh" "$plan_c2" >"$temporary_root/validate-resolved.log" 2>&1; then
    :
elif grep -Fq 'Adversarial review has unresolved findings' "$temporary_root/validate-resolved.log"; then
    fail 'validate-plan.sh reports unresolved findings for a resolved 5-column row'
fi

# --- W15: approval hook ---
plan_d="$temporary_root/plan-d"
seed_gated_plan "$plan_d" test-session-d
seed_secret_session test-session-d
"$script_dir/mint-fix-keys.sh" "$plan_d" >/dev/null 2>&1
key_d1="$(expected_key test-session-d AR-01 W05)"
write_claims "$plan_d" "AR-01	W05	$key_d1" "AR-03	W07	badkey"
if "$script_dir/update-plan-content.sh" --review-status "$plan_d" approved \
    >"$temporary_root/approve-forged.log" 2>&1; then
    fail 'approval accepted forged fix key claims'
fi
grep -Fq 'fix-keys verification failed' "$temporary_root/approve-forged.log" \
    || fail 'approval did not report the fix-keys verification failure'
grep -Fqx -- '- Status: `💤 pending`' "$plan_d/adversarial-review.md" \
    || fail 'refused approval changed the review status'
[ -d "$TMPDIR/planning-agent/review-fix-keys/test-session-d" ] \
    || fail 'refused approval invalidated the session secret'
write_claims "$plan_d" "AR-01	W05	$key_d1" "AR-03	W07	$(expected_key test-session-d AR-03 W07)"
# The gate names the claiming session: unnamed defaults to the minting session,
# which is self-certification and must be refused before the status flips.
if "$script_dir/update-plan-content.sh" --review-status "$plan_d" approved \
    >"$temporary_root/approve-selfcert.log" 2>&1; then
    fail 'approval accepted matching claims recorded by the minting session (self-certification)'
fi
grep -Fq 'self-certification' "$temporary_root/approve-selfcert.log" \
    || fail 'approval did not report self-certification'
grep -Fqx -- '- Status: `💤 pending`' "$plan_d/adversarial-review.md" \
    || fail 'refused self-certified approval changed the review status'
[ -d "$TMPDIR/planning-agent/review-fix-keys/test-session-d" ] \
    || fail 'refused self-certified approval invalidated the session secret'
CLAIMED_BY=fixer-session-d "$script_dir/update-plan-content.sh" --review-status "$plan_d" approved >/dev/null 2>&1 \
    || fail 'approval rejected matching fix key claims from a distinct claiming session'
grep -Fqx -- '- Status: `✅ approved`' "$plan_d/adversarial-review.md" \
    || fail 'review status was not flipped to approved'
grep -Fqx -- '- Status: ✅ approved' "$plan_d/plan-description.md" \
    || fail 'plan description did not mirror the approved status'
[ -f "$plan_d/fix-keys.json" ] || fail 'fix-keys.json must remain after approval (gated-state marker)'
[ -d "$TMPDIR/planning-agent/review-fix-keys/test-session-d" ] \
    && fail 'approval did not invalidate the session secret'
if "$script_dir/update-plan-content.sh" --review-status "$plan_d" approved \
    >"$temporary_root/reapprove.log" 2>&1; then
    fail 're-approval succeeded after the session was invalidated'
fi
grep -Fq 'session secret missing' "$temporary_root/reapprove.log" \
    || fail 're-approval did not fail closed on the invalidated session'
if "$script_dir/update-plan-content.sh" --review-status "$plan_d" pending >/dev/null 2>&1; then
    :
else
    fail 'pending transition failed after approval'
fi
if "$script_dir/update-plan-content.sh" --review-status "$plan_d" approved \
    >/dev/null 2>&1; then
    fail 're-approval succeeded after re-pending an invalidated gated plan'
fi

plan_e="$temporary_root/plan-e"
mkdir -p "$plan_e"
printf '# Plan: fixture\n\n- Status: `💤 pending`\n' > "$plan_e/plan-description.md"
{
    printf '# Adversarial review: fixture\n\n'
    printf '## Review scope\n\n'
    printf '## Findings\n\n'
    printf '| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n'
    printf '|---|---|---|---|---|\n'
    printf '| AR-01 | Gap. | Do it. | ✅ resolved | N/A |\n\n'
    printf 'No additional substantive finding remains.\n\n'
    printf '## Verdict\n\n'
    printf '%s\n' '- Status: `💤 pending`'
} > "$plan_e/adversarial-review.md"
"$script_dir/update-plan-content.sh" --review-status "$plan_e" approved >/dev/null 2>&1 \
    || fail 'an ungated plan (no fix-keys.json) did not approve without verification'
grep -Fqx -- '- Status: ✅ approved' "$plan_e/plan-description.md" \
    || fail 'ungated approval did not mirror the status'

plan_f="$temporary_root/plan-f"
seed_gated_plan "$plan_f" test-session-f
seed_secret_session test-session-f
"$script_dir/mint-fix-keys.sh" "$plan_f" >/dev/null 2>&1
rm -f "$plan_f/fixes.md"
if "$script_dir/update-plan-content.sh" --review-status "$plan_f" approved \
    >"$temporary_root/approve-nofixes.log" 2>&1; then
    fail 'approval accepted a gated plan without fixes.md'
fi
grep -Fq 'fixes.md missing' "$temporary_root/approve-nofixes.log" \
    || fail 'missing fixes.md was not reported at approval'
: > "$plan_f/fixes.md"
if "$script_dir/update-plan-content.sh" --review-status "$plan_f" approved \
    >"$temporary_root/approve-emptyfixes.log" 2>&1; then
    fail 'approval accepted a gated plan with an empty fixes.md'
fi

plan_g="$temporary_root/plan-g"
seed_gated_plan "$plan_g" test-session-g
seed_secret_session test-session-g
t_sed_i 's#| AR-01 | First gap. | Implement W05. | ✅ resolved | W05 |#| AR-01 | First gap. | Implement W05. | ✅ resolved | N/A |#; s#| AR-03 | Third gap. | Implement W07. | ✅ resolved | W07 |#| AR-03 | Third gap. | Implement W07. | ✅ resolved | N/A |#' \
    "$plan_g/adversarial-review.md"
"$script_dir/mint-fix-keys.sh" "$plan_g" >/dev/null 2>&1
"$script_dir/update-plan-content.sh" --review-status "$plan_g" approved >/dev/null 2>&1 \
    || fail 'a gated plan with no work units did not approve without verification'
[ -d "$TMPDIR/planning-agent/review-fix-keys/test-session-g" ] \
    && fail 'no-unit approval did not invalidate the session secret'

# --- P0-2: an approval that dies on a malformed document must leave the session
#     secret intact, so the review stays approvable ---
plan_h="$temporary_root/plan-h"
seed_gated_plan "$plan_h" test-session-h
seed_secret_session test-session-h
"$script_dir/mint-fix-keys.sh" "$plan_h" >/dev/null 2>&1
write_claims "$plan_h" "AR-01	W05	$(expected_key test-session-h AR-01 W05)" \
    "AR-03	W07	$(expected_key test-session-h AR-03 W07)"
t_sed_i '/^- Status:/d' "$plan_h/plan-description.md"
if CLAIMED_BY=fixer-session-h "$script_dir/update-plan-content.sh" --review-status "$plan_h" approved \
    >"$temporary_root/approve-nostatus.log" 2>&1; then
    fail 'approval succeeded with no Status field in plan-description.md'
fi
grep -Fq 'Plan description must contain exactly one Status field' "$temporary_root/approve-nostatus.log" \
    || fail 'approval did not report the malformed plan description'
[ -d "$TMPDIR/planning-agent/review-fix-keys/test-session-h" ] \
    || fail 'an approval that failed its writes invalidated the session secret'
printf '# Plan: fixture\n\n- Status: `💤 pending`\n' > "$plan_h/plan-description.md"
CLAIMED_BY=fixer-session-h "$script_dir/update-plan-content.sh" --review-status "$plan_h" approved \
    >"$temporary_root/approve-repaired.log" 2>&1 \
    || fail 'approval on a repaired description failed: the failed attempt was unrecoverable'
grep -Fqx -- '- Status: `✅ approved`' "$plan_h/adversarial-review.md" \
    || fail 'the repaired approval did not flip the review status'
grep -Fqx -- '- Status: ✅ approved' "$plan_h/plan-description.md" \
    || fail 'the repaired approval did not mirror the status into the description'
[ -d "$TMPDIR/planning-agent/review-fix-keys/test-session-h" ] \
    && fail 'the repaired approval did not invalidate the session secret'

t_end
