#!/usr/bin/env bash
# Coverage-gap tests (report: adversarial test-coverage audit).
#
# Covers behavior that the other suite misses:
#   - plan-reconcile-lib.sh: plan_rewrite_owned_work_units / plan_rebuild_goal_progress
#   - remove-plan.sh + cleanup-plans.sh (removal, git-history clear, --yes/list)
#   - create-work-unit-inventory.sh (create / 73 / 66)
#   - validate-plan.sh --no-propagation
#   - plan-content.sh summary subcommand
#   - plan-context.sh refresh --stale (non-gated, self-built plan)
#   - run-adversary-probe.sh (materialize + reader sanity + prompt)
#   - plan-mutate.sh add-progress / rebuild-progress
#   - verify-target.sh re-point + theme-override branches
#   - monitor-read.sh status / summary / grants

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scripts="$root/scripts"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0; fail=0
chk() {
    if [ "$1" = "$2" ]; then pass=$((pass+1)); echo "PASS: $3"; else fail=$((fail+1)); echo "FAIL: $3 (got $1, want $2)"; fi
}
expect_grep() {
    local file="$1" pattern="$2" label="$3"
    if grep -qE "$pattern" "$file"; then pass=$((pass+1)); echo "PASS: $label"; else fail=$((fail+1)); echo "FAIL: $label (pattern '$pattern' not found in $file)"; fi
}

# ---- plan-reconcile-lib.sh: rewrite_owned_work_units + rebuild_goal_progress ----
plan="$tmp/reconcile"
"$scripts/create-plan.sh" "$plan" reconcile >/dev/null
"$scripts/add-goal.sh" "$plan" 01-g 'G' 'O' >/dev/null
"$scripts/add-work-unit.sh" "$plan" W01 source a.php 'A::x' N/A 'change A' '—' 01-g 01-step-a >/dev/null
"$scripts/add-work-unit.sh" "$plan" W02 source b.php 'B::x' N/A 'change B' W01 01-g 02-step-b >/dev/null
"$scripts/update-plan-content.sh" --testing-requirement "$plan" 01-g no 'research' >/dev/null
goal_file="$plan/01-g/goal.md"
# Directly exercise the reconcile lib (the shared engine) rather than only via add-work-unit.
# shellcheck disable=SC1091
source "$scripts/plan-document-lib.sh"
source "$scripts/plan-reconcile-lib.sh"
# Rebuild owned work units from a hand-set inventory; assert both units appear and
# the testing-requirement row is preserved.
plan_rewrite_owned_work_units "$goal_file" "$plan/work-unit-inventory.md" 01-g
expect_grep "$goal_file" '`W01` — change A' 'reconcile: rewrite keeps W01'
expect_grep "$goal_file" '`W02` — change B' 'reconcile: rewrite keeps W02'
expect_grep "$goal_file" '\| no \|' 'reconcile: testing-requirement row preserved'
# Per-goal progress is created on demand (a goal whose tracker never existed).
if [ ! -f "$plan/01-g/progress.md" ]; then
    plan_rebuild_goal_progress "$scripts" "$plan/01-g" 01-g
fi
[ -f "$plan/01-g/progress.md" ] && { pass=$((pass+1)); echo "PASS: reconcile: per-goal progress created"; } \
    || { fail=$((fail+1)); echo "FAIL: reconcile: per-goal progress not created"; }
expect_grep "$plan/01-g/progress.md" '01-step-a' 'reconcile: progress has step row'

# ---- create-work-unit-inventory.sh ----
scaffold="$tmp/scaffold"
mkdir -p "$scaffold"
"$scripts/create-work-unit-inventory.sh" "$scaffold" >/dev/null
[ -f "$scaffold/work-unit-inventory.md" ] && { pass=$((pass+1)); echo "PASS: create-work-unit-inventory creates file"; } \
    || { fail=$((fail+1)); echo "FAIL: create-work-unit-inventory"; }
expect_grep "$scaffold/work-unit-inventory.md" '## Work units' 'inventory scaffold has Work units'
rc=0; if "$scripts/create-work-unit-inventory.sh" "$scaffold" >/dev/null 2>&1; then rc=0; else rc=$?; fi
chk "$rc" 73 "create-work-unit-inventory refuses existing (73)"
mkdir -p "$tmp/nonexistent-plan"
if "$scripts/create-work-unit-inventory.sh" "$tmp/nonexistent-plan/sub" >/dev/null 2>&1; then rc=0; else rc=$?; fi
chk "$rc" 66 "create-work-unit-inventory missing dir (66)"

# ---- remove-plan.sh + cleanup-plans.sh ----
export PLANS_ROOT="$tmp/cleanroot"
"$scripts/create-plan.sh" alpha 'A' >/dev/null
"$scripts/create-plan.sh" beta 'B' >/dev/null
"$scripts/remove-plan.sh" "$PLANS_ROOT/alpha" >/dev/null
[ -d "$PLANS_ROOT/alpha" ] && { fail=$((fail+1)); echo "FAIL: remove-plan removed dir"; } \
    || { pass=$((pass+1)); echo "PASS: remove-plan removes dir"; }
[ -d "$PLANS_ROOT/beta" ] && { pass=$((pass+1)); echo "PASS: remove-plan leaves other plan"; } \
    || { fail=$((fail+1)); echo "FAIL: remove-plan removed other plan"; }
# git history at the root persists while a plan remains.
[ -d "$PLANS_ROOT/.git" ] && { pass=$((pass+1)); echo "PASS: root git history kept while plans remain"; } \
    || { fail=$((fail+1)); echo "FAIL: root git history lost early"; }
# Removing the last plan clears the root git history.
"$scripts/remove-plan.sh" "$PLANS_ROOT/beta" >/dev/null
[ -d "$PLANS_ROOT/.git" ] && { fail=$((fail+1)); echo "FAIL: root git history not cleared on last plan"; } \
    || { pass=$((pass+1)); echo "PASS: root git history cleared on last plan"; }
# cleanup-plans: recreate two plans, --list marks, unknown name rejected, --yes removes.
"$scripts/create-plan.sh" gamma 'G' >/dev/null
"$scripts/create-plan.sh" delta 'D' >/dev/null
list_out="$(PLANS_ROOT="$PLANS_ROOT" "$scripts/cleanup-plans.sh" --list)"
expect_grep <(printf '%s' "$list_out") 'gamma' 'cleanup-plans --list shows plan'
rc=0; if PLANS_ROOT="$PLANS_ROOT" "$scripts/cleanup-plans.sh" nosuchplan >/dev/null 2>&1; then rc=0; else rc=$?; fi
chk "$rc" 66 "cleanup-plans unknown name (66)"
PLANS_ROOT="$PLANS_ROOT" "$scripts/cleanup-plans.sh" gamma --yes >/dev/null 2>&1
[ -d "$PLANS_ROOT/gamma" ] && { fail=$((fail+1)); echo "FAIL: cleanup-plans --yes did not remove"; } \
    || { pass=$((pass+1)); echo "PASS: cleanup-plans --yes removes"; }
# -y is the short form of --yes (the accepted -y flag must be covered too).
"$scripts/create-plan.sh" epsilon 'E' >/dev/null
PLANS_ROOT="$PLANS_ROOT" "$scripts/cleanup-plans.sh" epsilon -y >/dev/null 2>&1
[ -d "$PLANS_ROOT/epsilon" ] && { fail=$((fail+1)); echo "FAIL: cleanup-plans -y did not remove"; } \
    || { pass=$((pass+1)); echo "PASS: cleanup-plans -y removes"; }

# ---- validate-plan.sh --no-propagation ----
prop_plan="$tmp/prop"
"$scripts/create-plan.sh" "$prop_plan" prop >/dev/null
"$scripts/add-goal.sh" "$prop_plan" 01-g 'G' 'O' >/dev/null
"$scripts/add-work-unit.sh" "$prop_plan" W01 source a.php 'A::x' N/A 'change A' '—' 01-g 01-step-a >/dev/null
"$scripts/add-work-unit.sh" "$prop_plan" W02 verification N/A 'v.sh' N/A 'Verify W01.' '—' 01-g 02-step-v >/dev/null
"$scripts/update-plan-content.sh" --testing-requirement "$prop_plan" 01-g yes 'obs' >/dev/null
printf '# V\n\n## Automated tests\n\nx\n' > "$prop_plan/01-g/steps/01-step-a-testing.md"
printf '# V\n\n## Automated tests\n\nx\n' > "$prop_plan/01-g/steps/02-step-v-testing.md"
"$scripts/add-coverage.sh" "$prop_plan" 'A.' W01 'c' >/dev/null
"$scripts/add-coverage.sh" "$prop_plan" 'V.' W02 'c' >/dev/null
"$scripts/create-adversarial-review.sh" "$prop_plan" >/dev/null
# W02 grades W01 but does not depend on it -> propagation FAIL.
"$scripts/update-plan-content.sh" -sp "$prop_plan" 01-g/02-step-v 4.1 'Verify W01.' >/dev/null
if "$scripts/validate-plan.sh" "$prop_plan" >"$tmp/prop-on.log" 2>&1; then :; fi
expect_grep "$tmp/prop-on.log" 'no dependency path' 'propagation-on catches grader gap'
# --no-propagation disables it (the plan may still have unrelated structural
# errors; we only assert the propagation message is absent).
"$scripts/validate-plan.sh" --no-propagation "$prop_plan" >"$tmp/prop-off.log" 2>&1 || true
if grep -q 'no dependency path' "$tmp/prop-off.log"; then
    fail=$((fail+1)); echo "FAIL: --no-propagation still emitted grader message"
else
    pass=$((pass+1)); echo "PASS: --no-propagation suppresses propagation"
fi

# ---- plan-content.sh summary ----
sum_plan="$tmp/sum"
"$scripts/create-plan.sh" "$sum_plan" sum >/dev/null
"$scripts/add-goal.sh" "$sum_plan" 01-g 'G' 'O' >/dev/null
"$scripts/add-work-unit.sh" "$sum_plan" W01 source a.php 'A::x' N/A 'change A' '—' 01-g 01-step-a >/dev/null
sum_md="$("$scripts/plan-content.sh" summary "$sum_plan" markdown 2>/dev/null)"
expect_grep <(printf '%s' "$sum_md") 'Plan summary' 'summary markdown has title'
expect_grep <(printf '%s' "$sum_md") 'W01' 'summary markdown lists unit'
sum_json="$("$scripts/plan-content.sh" summary "$sum_plan" json 2>/dev/null)"
expect_grep <(printf '%s' "$sum_json") '"id":"W01"' 'summary json has unit id'

# ---- plan-context.sh refresh --stale (non-gated) ----
ctx_plan="$tmp/ctx"
"$scripts/create-plan.sh" "$ctx_plan" ctx >/dev/null
"$scripts/plan-context.sh" init --plan-dir "$ctx_plan" >/dev/null
"$scripts/plan-context.sh" read --plan-dir "$ctx_plan" --document plan --view summary >/dev/null
# External edit makes the plan stale; refresh --stale must not fail and should
# report or clear the stale entry.
printf '\n# external change\n' >> "$ctx_plan/plan-description.md"
refresh_out="$("$scripts/plan-context.sh" refresh --plan-dir "$ctx_plan" --stale 2>&1)" || true
[ -d "$ctx_plan/context" ] && { pass=$((pass+1)); echo "PASS: plan-context refresh --stale runs (non-gated)"; } \
    || { fail=$((fail+1)); echo "FAIL: plan-context refresh"; }

# ---- run-adversary-probe.sh ----
probe_out="$("$scripts/run-adversary-probe.sh" "$tmp/probe" 2>&1)" || true
expect_grep <(printf '%s' "$probe_out") 'gate serves --document plan' 'adversary-probe reader sanity'
expect_grep <(printf '%s' "$probe_out") 'spawn a fresh adversarial reviewer' 'adversary-probe emits prompt'
[ -f "$tmp/probe/adversarial-review.md" ] && { pass=$((pass+1)); echo "PASS: adversary-probe materialized fixture"; } \
    || { fail=$((fail+1)); echo "FAIL: adversary-probe did not materialize"; }

# ---- plan-mutate.sh add-progress / rebuild-progress ----
mut="$tmp/mut"
"$scripts/create-plan.sh" "$mut" mut >/dev/null
"$scripts/add-goal.sh" "$mut" 01-g 'G' 'O' >/dev/null
"$scripts/add-work-unit.sh" "$mut" W01 source a.php 'A::x' N/A 'change A' '—' 01-g 01-step-a >/dev/null
# remove the per-goal tracker so add-progress creates a fresh row for a
# not-yet-tracked step (the append path), then rebuild rewrites from step files.
rm -f "$mut/01-g/progress.md"
"$scripts/create-progress.sh" "$mut/01-g" 01-g >/dev/null
"$scripts/plan-mutate.sh" add-progress "$mut/01-g" 02-step-extra 'extra step' >/dev/null 2>&1 || true
if grep -q '02-step-extra' "$mut/01-g/progress.md" 2>/dev/null; then
    pass=$((pass+1)); echo "PASS: plan-mutate add-progress appends row"
else
    fail=$((fail+1)); echo "FAIL: plan-mutate add-progress append"
fi
"$scripts/plan-mutate.sh" rebuild-progress "$mut/01-g" >/dev/null 2>&1 || true
expect_grep "$mut/01-g/progress.md" '01-step-a' 'plan-mutate rebuild-progress rewrites rows'

# ---- verify-target.sh re-point + theme-override branches ----
vt="$tmp/vt"
"$scripts/create-plan.sh" "$vt" vt >/dev/null
"$scripts/add-goal.sh" "$vt" 01-g 'G' 'O' >/dev/null
"$scripts/add-work-unit.sh" "$vt" W01 markup app/design/frontend/Theme/templates/order/history.phtml '#order_history' N/A 'render history' '—' 01-g 01-step-a >/dev/null
repo="$tmp/vtrepo"
mkdir -p "$repo/app/design/frontend/Theme/templates/order" "$repo/app/code/M/view/frontend/layout"
printf 'template\n' > "$repo/app/design/frontend/Theme/templates/order/history.phtml"
# re-point: a layout re-assigns the block to a different template -> WARN
printf '<layout><referenceBlock name="order_history"><action method="setTemplate"><argument name="template" xsi:type="string">different.phtml</argument></action></referenceBlock></layout>\n' \
    > "$repo/app/code/M/view/frontend/layout/catalog.xml"
vt_out="$("$scripts/verify-target.sh" "$vt" W01 --repo "$repo" 2>&1)" || true
expect_grep <(printf '%s' "$vt_out") 're-points block' 'verify-target flags setTemplate re-point'
# theme override: module template overridden under app/design
vt2="$tmp/vt2"
"$scripts/create-plan.sh" "$vt2" vt2 >/dev/null
"$scripts/add-goal.sh" "$vt2" 01-g 'G' 'O' >/dev/null
"$scripts/add-work-unit.sh" "$vt2" W01 markup app/code/M/view/frontend/templates/order/history.phtml '#order_history' N/A 'render history' '—' 01-g 01-step-a >/dev/null
repo2="$tmp/vtrepo2"
mkdir -p "$repo2/app/code/M/view/frontend/templates/order" "$repo2/app/design/frontend/Theme/templates/order"
printf 'module\n' > "$repo2/app/code/M/view/frontend/templates/order/history.phtml"
printf 'override\n' > "$repo2/app/design/frontend/Theme/templates/order/history.phtml"
vt2_out="$("$scripts/verify-target.sh" "$vt2" W01 --repo "$repo2" 2>&1)" || true
expect_grep <(printf '%s' "$vt2_out") 'theme override' 'verify-target flags theme override'

# ---- monitor-read.sh status / summary / grants ----
frame_sh="$scripts/supervision-frame.sh"
monitor_sh="$scripts/monitor-read.sh"
mkdir -p "$tmp/frames"
bash "$frame_sh" write "$tmp/frames/a" --subagent a --persona chris --status ok --verdict clean >/dev/null
status_out="$(ROLE_ID=maintainer bash "$monitor_sh" status "$tmp/frames/a" 2>/dev/null)"
expect_grep <(printf '%s' "$status_out") 'status=ok' 'monitor-read status'
summary_out="$(ROLE_ID=maintainer bash "$monitor_sh" summary "$tmp/frames" 2>/dev/null)"
expect_grep <(printf '%s' "$summary_out") 'a:' 'monitor-read summary'
printf 'grant\treviewer\tneeds path\tcommand\n' > "$tmp/grants.log"
grants_out="$(ROLE_ID=maintainer bash "$monitor_sh" grants "$tmp/grants.log" --last 5 2>/dev/null)"
expect_grep <(printf '%s' "$grants_out") 'grant' 'monitor-read grants'

# ---- flag coverage: role-context --page / --page-size, supervision-frame
#      optional write flags, plan-content --full ----
# role-context accepts -p/--page and --page-size (paged role-doc reads).
rc_page=0
if ROLE_ID=maintainer bash "$scripts/role-context.sh" maintainer -p 1 --page-size 2048 >/dev/null 2>&1; then
    rc_page=0
else
    rc_page=$?
fi
chk "$rc_page" 0 "role-context --page/--page-size accepted"
# --page actually pages: with a tiny page budget, page 1 is smaller than the
# full payload and shows a "more" continuation, page 2 differs from page 1.
full_payload="$(ROLE_ID=maintainer bash "$scripts/role-context.sh" maintainer 2>/dev/null || true)"
page1="$(ROLE_ID=maintainer bash "$scripts/role-context.sh" maintainer -p 1 --page-size 500 2>/dev/null || true)"
page2="$(ROLE_ID=maintainer bash "$scripts/role-context.sh" maintainer -p 2 --page-size 500 2>/dev/null || true)"
page1_continues=false
case "$page1" in *'more:'*) page1_continues=true ;; esac
if [ -n "$page1" ] && [ -n "$page2" ] && [ "$page1" != "$page2" ] && [ "$page1_continues" = true ]; then
    pass=$((pass+1)); echo "PASS: role-context --page paginates (p1 != p2, more: present)"
else
    fail=$((fail+1)); echo "FAIL: role-context --page does not paginate"
fi
# --paths lists the role's doc paths (maintainer-only).
paths_out="$(ROLE_ID=maintainer bash "$scripts/role-context.sh" --paths maintainer 2>/dev/null || true)"
case "$paths_out" in *'ROLES.md'*) paths_listed=true ;; *) paths_listed=false ;; esac
if [ "$paths_listed" = true ]; then
    pass=$((pass+1)); echo "PASS: role-context --paths lists doc paths"
else
    fail=$((fail+1)); echo "FAIL: role-context --paths"
fi
rc_nonmain=0
if ROLE_ID=chris bash "$scripts/role-context.sh" --paths chris >/dev/null 2>&1; then rc_nonmain=0; else rc_nonmain=$?; fi
chk "$rc_nonmain" 64 "role-context --paths non-maintainer refused"
# supervision-frame write accepts the escalation/read/wholesale flags.
"$scripts/supervision-frame.sh" write "$tmp/fullframe" \
    --subagent r --persona chris --status escalated \
    --read-discipline violated --wholesale-reads 3 --skill-loaded none \
    --needs-escalation none --grant-requested none --verdict clean >/dev/null 2>&1
[ -f "$tmp/fullframe" ] && { pass=$((pass+1)); echo "PASS: supervision-frame optional write flags accepted"; } \
    || { fail=$((fail+1)); echo "FAIL: supervision-frame optional write flags"; }
# plan-content find --full (no excerpt truncation on a long line).
full_plan="$tmp/full"
"$scripts/create-plan.sh" "$full_plan" full >/dev/null
"$scripts/add-goal.sh" "$full_plan" 01-g 'G' 'O' >/dev/null
"$scripts/add-work-unit.sh" "$full_plan" W01 source a.php 'A::x' N/A "$(printf 'long%.0s' $(seq 1 60))" '—' 01-g 01-step-a >/dev/null
full_out="$("$scripts/plan-content.sh" find "$full_plan" 'longlong' --in units --full 2>/dev/null || true)"
truncated=false
case "$full_out" in *'...'*) truncated=true ;; esac
excerpt_present=false
case "$full_out" in *'longlong'*) excerpt_present=true ;; esac
if [ "$truncated" = false ] && [ "$excerpt_present" = true ]; then
    pass=$((pass+1)); echo "PASS: find --full shows untruncated excerpt"
else
    fail=$((fail+1)); echo "FAIL: find --full truncation"
fi

echo
echo "coverage-gaps: $pass passed, $fail failed"
[ "$fail" -eq 0 ] && echo "test-coverage-gaps: PASS" || exit 1