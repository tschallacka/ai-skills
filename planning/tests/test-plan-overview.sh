#!/usr/bin/env bash
# MODE: DEV
# test-plan-overview.sh — the plan overview renderer.
#
# Pins: byte-deterministic output under OVERVIEW_NOW; a fully self-contained
# document (no external src/href/url, every template token substituted);
# state derivation (planning → implementation → delivered) driven by the real
# review gate and step statuses; chart and narration structure; refusals.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-overview-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

fail() { t_fail "$*"; }
NOW=2026-08-22T1200Z

# A bare setup command under set -e dies with its diagnosis discarded, which is
# how a macOS-only failure read as a silent exit 1 (B31). Setup runs through
# here: on failure the captured output prints before the exit.
must() {
    local label="$1" rc=0 tmp
    shift
    tmp="$(mktemp "${TMPDIR:-/tmp}/overview-must.XXXXXX")"
    OVERVIEW_NOW=$NOW "$@" >"$tmp" 2>&1 || rc=$?
    if [ "$rc" -ne 0 ]; then
        sed 's/^/  | /' "$tmp" >&2
        rm -f "$tmp"
        printf 'test-plan-overview: %s exited %d\n' "$label" "$rc" >&2
        t_end
        exit "$rc"
    fi
    rm -f "$tmp"
}

seed_plan() { # <plan-dir>
    local plan="$1"
    "$script_dir/create-plan.sh" "$plan" 'Overview fixture' >/dev/null
    "$script_dir/add-goal.sh" "$plan" 01-build 'Build' 'A thing' >/dev/null
    "$script_dir/add-work-unit.sh" "$plan" W01 source a.php 'A::x' N/A 'change A' '—' 01-build 01-step-a >/dev/null
    "$script_dir/add-work-unit.sh" "$plan" W02 verification N/A v.sh N/A 'Verify A' 'W01' 01-build 02-step-v >/dev/null
    "$script_dir/update-plan-content.sh" --testing-requirement "$plan" 01-build yes 'verification unit present' >/dev/null
    printf '# t\n\n## Automated tests\n\nx\n' > "$plan/01-build/steps/01-step-a-testing.md"
    printf '# t\n\n## Automated tests\n\nx\n' > "$plan/01-build/steps/02-step-v-testing.md"
    "$script_dir/create-adversarial-review.sh" "$plan" >/dev/null
}

# --- fresh plan: planning state, placeholder chart, self-contained -----------
plan="$temporary_root/fresh"
seed_plan "$plan"
rc=0
OVERVIEW_NOW=$NOW "$script_dir/render-plan-overview.sh" "$plan" --refresh 7 >/dev/null 2>&1 || rc=$?
t_assert_eq "fresh render exits 0" "$rc" 0
out="$plan/overview.html"
[ -f "$out" ] || { fail "no overview.html produced"; t_end; exit 0; }
t_assert_eq "html tag carries planning state" "$(grep -c '^<html[^>]*data-state="planning"' "$out")" 1
t_assert_eq "no unsubstituted tokens remain" "$(grep -c '@[A-Z_]*@' "$out" || true)" 0
t_assert_eq "no external asset references" \
    "$(grep -cE '(src|href)="https?:|url\(https?:' "$out" || true)" 0
t_assert_eq "placeholder cycle chart present with zero cycles" \
    "$(grep -c 'no review cycles recorded yet' "$out")" 1
t_assert_eq "meta refresh honours --refresh" "$(grep -c '"refresh" content="7"' "$out")" 1

# --out writes where it is told, leaving the plan untouched.
rc=0
OVERVIEW_NOW=$NOW "$script_dir/render-plan-overview.sh" "$plan" --out "$temporary_root/custom.html" >/dev/null 2>&1 || rc=$?
t_assert_eq "--out render exits 0" "$rc" 0
[ -f "$temporary_root/custom.html" ] || fail "--out produced no file"
cp "$out" "$temporary_root/before-out.html"
must 'render --out custom2' "$script_dir/render-plan-overview.sh" "$plan" --out "$temporary_root/custom2.html"
cmp -s "$temporary_root/before-out.html" "$out" || fail "--out run also rewrote the plan's overview.html"

# --watch regenerates when an input changes (2 s poll, one forced edit).
wd="$temporary_root/watched"
cp -R "$plan" "$wd"
rm -f "$wd/overview.html"
OVERVIEW_NOW=$NOW timeout 9 "$script_dir/render-plan-overview.sh" "$wd" --watch 2 --refresh 5 \
    >"$temporary_root/watch.log" 2>&1 &
wpid=$!
sleep 3
printf '\n# touched\n' >> "$wd/work-unit-inventory.md"
wait $wpid || true
renders="$(grep -c '^Rendered ' "$temporary_root/watch.log")"
if [ "$renders" -lt 2 ]; then fail "--watch did not re-render on change ($renders renders)"; fi

# determinism: same inputs, pinned clock, byte-identical bytes.
cp "$out" "$temporary_root/first.html"
must 'determinism render' "$script_dir/render-plan-overview.sh" "$plan" --refresh 7
cmp -s "$temporary_root/first.html" "$out" || fail "output is not deterministic under OVERVIEW_NOW"

# --- implementation: approved review plus partial steps ----------------------
must 'add finding AR-05' "$script_dir/add-adversarial-finding.sh" "$plan" AR-05 'Thin scope' 'Narrow it' resolved --work-unit W01
MINTED_BY=test-reviewer must 'mint fix keys' "$script_dir/mint-fix-keys.sh" "$plan"
k="$(jq -r '.keys["AR-05"]["W01"]' "$plan/fix-keys.json")"
must 'add fix claim' "$script_dir/add-fix-claim.sh" "$plan" --finding AR-05 --work-unit W01 --key "$k"
must 'review approved' "$script_dir/update-plan-content.sh" --review-status "$plan" approved
must 'step 01-step-a completed' "$script_dir/update-step.sh" "$plan/01-build" 01-step-a completed
must 'land findings cycle 1' "$script_dir/update-adversarial-review.sh" "$plan" --cycle 1 <<'CSV'
ID,Missing or over-broad item,Required plan change,Status,Work unit
AR-05,Thin scope,Narrow it,✅ resolved,W01
CSV
must 'render implementation' "$script_dir/render-plan-overview.sh" "$plan"
t_assert_eq "approved review flips state to implementation" "$(grep -c '^<html[^>]*data-state="implementation"' "$out")" 1
t_assert_eq "step ledger counts data rows only (no header rows)" \
    "$(grep -o '<span class="go">' "$out" | wc -l | tr -d ' ')" 2
t_assert_eq "cycle chart drawn from history" "$(grep -c 'class="line-res"' "$out")" 1
t_assert_eq "narration reports the next step" "$(grep -c 'class="ln">[^<]*next up 02-step-v' "$out")" 1
t_assert_eq "findings counted without the header row" "$(grep -o 'findings <b>[0-9]*</b>' "$out" | head -1)" 'findings <b>1</b>'

# --- delivered: every step complete ------------------------------------------
must 'step 02-step-v completed' "$script_dir/update-step.sh" "$plan/01-build" 02-step-v completed
must 'render delivered' "$script_dir/render-plan-overview.sh" "$plan"
t_assert_eq "all steps done flips state to delivered" "$(grep -c '^<html[^>]*data-state="delivered"' "$out")" 1

# --- refusals ------------------------------------------------------------------
rc=0; OVERVIEW_NOW=$NOW "$script_dir/render-plan-overview.sh" "$temporary_root/nope" >/dev/null 2>&1 || rc=$?
t_assert_eq "missing plan refuses with 66" "$rc" 66
rc=0; OVERVIEW_NOW=$NOW "$script_dir/render-plan-overview.sh" "$plan" --bogus >/dev/null 2>&1 || rc=$?
t_assert_eq "unknown option refuses with 64" "$([ "$rc" -ne 0 ] && echo yes || echo no)" yes

t_end
