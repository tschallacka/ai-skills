#!/usr/bin/env bash
# MODE: DEV
# test-add-fix-claim.sh — fixes.md has a writer.
#
# Five scripts read fixes.md and nothing wrote it. The fixer was expected to
# produce it while SKILL.md forbids hand-authoring a plan artifact, so satisfying
# the fix-key gate meant breaking that rule. add-fix-claim.sh records a claim,
# and what it accepts has to agree with what verify-fix-keys.sh gates: the gated
# pairs come from the review's Findings table, not from fix-keys.json.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/add-fix-claim.XXXXXX")"
trap 'rm -rf "$work"' EXIT

FIXED_SECRET='00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff'
expected_key() { # <session> <finding> <work-unit>
    # Same derivation as the scripts under test: SHA-256 over SECRET||MESSAGE.
    printf '%s%s' "$FIXED_SECRET" "$1|$2|$3" | sha256sum | awk '{print $1}'
}

# verify-fix-keys derives the expected keys from the session secret, which lives
# outside the plan. Seed it with the same fixed secret the keys were built from.
seed_secret_session() {
    local dir="${TMPDIR:-/tmp}/planning-agent/review-fix-keys/$1"
    mkdir -p "$dir"
    printf '%s\n' "$FIXED_SECRET" > "$dir/secret"
    chmod 700 "$dir"
    chmod 600 "$dir/secret"
}

session=test-session-claim
seed_secret_session "$session"
plan="$work/plan"
mkdir -p "$plan"
printf '# Plan: fixture\n\n- Status: `💤 pending`\n' > "$plan/plan-description.md"
{
    printf '# Adversarial review: fixture\n\n'
    printf '## Review scope\n\n'
    printf '## Findings\n\n'
    printf '| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n'
    printf '|---|---|---|---|---|\n'
    printf '| AR-01 | First gap. | Implement W05. | ✅ resolved | W05 |\n'
    printf '| AR-02 | Second gap. | None. | ✅ resolved | N/A |\n'
    printf '## Verdict\n\n'
    printf '%s\n' '- Status: `💤 pending`'
} > "$plan/adversarial-review.md"

key_05="$(expected_key "$session" AR-01 W05)"
printf '{"session_id": "%s", "minted_by": "reviewer-1", "keys": {"AR-01|W05": "%s"}}\n' \
    "$session" "$key_05" > "$plan/fix-keys.json"

claim_line() { awk -F'\t' -v f="$1" -v w="$2" '$1 == f && $2 == w { print $3 }' "$plan/fixes.md"; }

# ---- the writer records a claim ---------------------------------------------
[ ! -f "$plan/fixes.md" ] || t_fail 'the fixture already has a fixes.md'
rc=0
"$scripts_dir/add-fix-claim.sh" "$plan" --finding AR-01 --work-unit W05 --key "$key_05" \
    >/dev/null 2>&1 || rc=$?
t_assert_eq 'a gated claim is recorded' "$rc" 0
[ -f "$plan/fixes.md" ] || t_fail 'fixes.md was not created'
t_assert_eq 'the claim carries the minted key' "$(claim_line AR-01 W05)" "$key_05"

# ---- and verify-fix-keys accepts what the writer produced -------------------
# The two must agree on the format, or a claim that writes cleanly still fails
# the gate. A distinct claimer, because the minting session is refused.
rc=0
verify_out="$("$scripts_dir/verify-fix-keys.sh" "$plan" --claimed-by fixer-session-2 2>&1)" || rc=$?
if [ "$rc" -ne 0 ]; then
    printf '%s\n' "$verify_out" >&2
    t_fail 'verify-fix-keys rejected a claim written by add-fix-claim'
fi

# ---- recording the same claim twice is a no-op, not a duplicate line --------
rc=0
"$scripts_dir/add-fix-claim.sh" "$plan" --finding AR-01 --work-unit W05 --key "$key_05" \
    >/dev/null 2>&1 || rc=$?
t_assert_eq 'a repeated identical claim succeeds' "$rc" 0
t_assert_eq 'and does not append a second line' \
    "$(grep -c . "$plan/fixes.md")" 1

# ---- a different key for a claimed pair is refused -------------------------
# Two lines for one pair would let verify pass on whichever it read first.
other_key="$(expected_key other-session AR-01 W05)"
# No `grep -q` in a pipeline: a match closes the pipe and the writer's SIGPIPE
# (141) is what pipefail reports. See PORTABILITY.md, pipefail-grep-q.
case "$other_key" in
    *[!0-9a-f]* | '') t_fail 'the probe key is malformed' ;;
esac
rc=0
"$scripts_dir/add-fix-claim.sh" "$plan" --finding AR-01 --work-unit W05 --key "$other_key" \
    >/dev/null 2>&1 || rc=$?
[ "$rc" -ne 0 ] || t_fail 'a second, different key was accepted for a pair already claimed'
t_assert_eq 'and fixes.md still holds one line' "$(grep -c . "$plan/fixes.md")" 1

# ---- an ungated pair is refused, not warned about later --------------------
rc=0
message="$("$scripts_dir/add-fix-claim.sh" "$plan" --finding AR-02 --work-unit W09 --key "$key_05" 2>&1)" || rc=$?
[ "$rc" -ne 0 ] || t_fail 'a pair the review does not gate was accepted'
case "$message" in
    *'not a gated pair'*) ;;
    *) t_fail "the refusal did not say the pair is ungated: $message" ;;
esac

# ---- a key that was never minted is refused -------------------------------
rc=0
message="$("$scripts_dir/add-fix-claim.sh" "$plan" --finding AR-01 --work-unit W05 \
    --key "$(printf 'a%.0s' $(seq 1 64))" 2>&1)" || rc=$?
[ "$rc" -ne 0 ] || t_fail 'a key absent from fix-keys.json was accepted'
case "$message" in
    *'not in fix-keys.json'*) ;;
    *) t_fail "the refusal did not name the reason: $message" ;;
esac

# ---- malformed ids and keys are refused -----------------------------------
# Each case is a complete argument list: appending a good --key after them let
# the later flag win, so the bad-key case proved nothing.
while IFS= read -r bad; do
    [ -n "$bad" ] || continue
    rc=0
    # shellcheck disable=SC2086
    "$scripts_dir/add-fix-claim.sh" "$plan" $bad >/dev/null 2>&1 || rc=$?
    [ "$rc" -ne 0 ] || t_fail "malformed arguments were accepted: $bad"
done <<MALFORMED
--finding AR1 --work-unit W05 --key $key_05
--finding AR-01 --work-unit 05 --key $key_05
--finding AR-01 --work-unit W05 --key nothex
--finding AR-01 --key $key_05
--work-unit W05 --key $key_05
MALFORMED

# ---- a refusal leaves no temp file in the plan ----------------------------
t_assert_eq 'no temp file was left behind' "$(find "$plan" -name '*.tmp.*' | wc -l | tr -d ' ')" 0

# ---- the plan directory is accepted both ways -----------------------------
plan_two="$work/plan-two"
cp -R "$plan" "$plan_two"
rm -f "$plan_two/fixes.md"
rc=0
"$scripts_dir/add-fix-claim.sh" --plan-dir "$plan_two" --finding AR-01 --work-unit W05 \
    --key "$key_05" >/dev/null 2>&1 || rc=$?
t_assert_eq '--plan-dir is accepted as a synonym' "$rc" 0

t_end
