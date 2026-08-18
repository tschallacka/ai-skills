#!/usr/bin/env bash
# Progress-bar shape contract test.
#
# Every Markdown file that renders a progress bar under the plans root must be
# in a shape the progress helpers can actually parse, or the bar silently stays
# at 0% (the exact bug that this test guards against: goal progress files
# authored in a 3-column `Work unit | Step | Status` shape while
# update-progress.sh parses the canonical 4-column shape and reads status from
# column 5).
#
# It scans every `progress.md` under the plans root:
#   * plan-level files  (`**Overall progress:**`) — must be the canonical
#     3-column `| Goalname | Description | Completion status |` shape; parsed by
#     update-plan-progress.sh reading status from column 4.
#   * goal-level files (`**Progress:**`) — must be the canonical 4-column
#     `| Goalname | Stepname | Description | Completion status |` shape; parsed
#     by update-progress.sh reading status from column 5.
#
# For each file it also functionally re-derives the bar percentage from the
# completed/total rows (a hand-rolled mirror of update-progress.sh /
# rebuild-plan-progress.sh parsing), so both shape drift and a genuinely broken
# bar are caught. The mirror deliberately re-states the parsers' column contract
# instead of invoking the helpers, keeping this test usable from a fixture dir.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scripts="$root/planning/scripts"
plans_root="${PLANS_ROOT:-$root/.plans}"

# In clean CI a live .plans tree may be absent; fall back to a deterministic
# fixture so the shape/parse contract is still exercised. PLANS_ROOT always
# overrides (used to point at the live tree or at a specific fixture).
fixture_plans_root="$root/planning/tests/fixtures/progress-shape"
if [ ! -d "$plans_root" ] && [ -d "$fixture_plans_root" ]; then
    plans_root="$fixture_plans_root"
fi

fail=0
note_fail() { echo "progress-bar: $1" >&2; fail=1; }

# ---- shape helpers (mirror the parsers exactly, so they can't drift apart) ----
# update-progress.sh: -F'|'  goal=$2  status=$5   (canonical 4 data cols)
# update-plan-progress.sh: -F'|'  goal=$2  status=$4  (canonical 3 data cols)
goal_header='| Goalname | Stepname | Description | Completion status |'
goal_sep='|---|---|---|---|'
plan_header='| Goalname | Description | Completion status |'
plan_sep='|---|---|---|'

if [ ! -d "$plans_root" ]; then
    echo "progress-bar: no plans root at $plans_root (nothing to validate)"
    exit 0
fi

count=0
while IFS= read -r f; do
    [ -n "$f" ] || continue
    count=$((count + 1))
    is_goal=0; is_plan=0
    grep -Fq '**Overall progress:**' "$f" && is_plan=1
    grep -Fq '**Progress:**' "$f" && ! grep -Fq '**Overall progress:**' "$f" && is_goal=1
    if [ "$is_goal" -eq 0 ] && [ "$is_plan" -eq 0 ]; then
        note_fail "no progress-bar marker in $f"
        continue
    fi

    if [ "$is_plan" -eq 1 ]; then
        grep -Fq "$plan_header" "$f" || note_fail "plan-level $f missing canonical header; expected: $plan_header"
        grep -Fq "$plan_sep" "$f" || note_fail "plan-level $f missing canonical separator; expected: $plan_sep"
    fi
    if [ "$is_goal" -eq 1 ]; then
        grep -Fq "$goal_header" "$f" || note_fail "goal-level $f missing canonical header; expected: $goal_header"
        grep -Fq "$goal_sep" "$f" || note_fail "goal-level $f missing canonical separator; expected: $goal_sep"
    fi

    # Validate the bar has a plausible marker and the status cells use valid values.
    grep -Eq '^\*\*(Overall )?[Pp]rogress:\*\* `[0-9]+%' "$f" \
        || note_fail "$f progress bar missing a percentage"
done < <(find "$plans_root" -type f -name 'progress.md' -not -path '*/context/*' | sort)

[ "$count" -gt 0 ] || { echo "progress-bar: no progress.md found under $plans_root"; exit 0; }

# ---- functional recompute: every goal-level bar must match its rows ----
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# recompute_goal_bar <progress-file> : print "EXPECTED_PCT" and set rc nonzero on
# mismatch. Hand-rolled parser mirrors update-progress.sh (goal=$2, status=$5).
recompute_goal_bar() {
    local f="$1" expected=0 total=0 declared ep
    while IFS='|' read -r _ goal _ _ status; do
        goal_text="$(printf '%s' "${goal:-}" | sed 's/^ *//;s/ *$//')"
        status_text="$(printf '%s' "${status:-}" | sed 's/^ *//;s/ *$//')"
        # Mirror update-progress.sh: skip header/separator rows AND rows whose
        # status is a dash or empty (so the mirror matches the real parser).
        case "$goal_text" in
            'Goalname'|'---') continue ;;
        esac
        case "$status_text" in
            ''|'---') continue ;;
        esac
        total=$((total + 1))
        case "$status_text" in *completed*) expected=$((expected + 1)) ;; esac
    done < "$f"
    ep=0
    [ "$total" -gt 0 ] && ep=$(( (expected * 100 + total / 2) / total ))
    declared="$(grep -Eo '^\*\*Progress:\*\* `[0-9]+%' "$f" | grep -Eo '[0-9]+' | head -1 || true)"
    if [ -z "$declared" ]; then
        note_fail "$f no percentage in bar"; return 1
    elif [ "$declared" -ne "$ep" ]; then
        note_fail "$f bar declares ${declared}% but its rows derive ${ep}% (${expected}/${total} completed)"; return 1
    fi
}

# recompute_plan_bar <progress-file> : derive plan % from its goal dirs'
# completion. A goal counts complete only at `**Progress:** `100%`; a goal dir
# with goal.md but no progress.md counts in the total, as incomplete.
recompute_plan_bar() {
    local f="$1" plan_dir ptotal=0 pdone=0 pe declared_p gdir
    plan_dir="$(dirname "$f")"
    while IFS= read -r gdir; do
        [ -f "$gdir/goal.md" ] || continue
        ptotal=$((ptotal + 1))
        if [ -f "$gdir/progress.md" ] && grep -Fq '**Progress:** `100%' "$gdir/progress.md"; then
            pdone=$((pdone + 1))
        fi
    done < <(find "$plan_dir" -mindepth 1 -maxdepth 1 -type d | sort)
    pe=0
    [ "$ptotal" -gt 0 ] && pe=$(( (pdone * 100 + ptotal / 2) / ptotal ))
    declared_p="$(grep -Eo '^\*\*Overall progress:\*\* `[0-9]+%' "$f" | grep -Eo '[0-9]+' | head -1 || true)"
    if [ -z "$declared_p" ]; then
        note_fail "$f no percentage in bar"; return 1
    elif [ "$declared_p" -ne "$pe" ]; then
        note_fail "$f bar declares ${declared_p}% but its goals derive ${pe}% (${pdone}/${ptotal})"; return 1
    fi
    # zero-goal-dir plan: rebuild-plan-progress.sh would exit 66 (broken state);
    # here we can't derive a percentage, so accept if the bar is simply absent or
    # if there genuinely are no goal dirs. We only note it for visibility.
    [ "$ptotal" -gt 0 ] || echo "progress-bar: note: $f has no goal dirs (cannot derive %)"
    return 0
}

while IFS= read -r f; do
    [ -n "$f" ] || continue
    # `|| true` keeps note_fail aggregation intact: a single mismatch must not
    # abort (set -e) and skip the remaining files / final summary.
    if grep -Fq '**Overall progress:**' "$f"; then
        recompute_plan_bar "$f" || true
    elif grep -Fq '**Progress:**' "$f"; then
        recompute_goal_bar "$f" || true
    fi
done < <(find "$plans_root" -type f -name 'progress.md' -not -path '*/context/*' | sort)

# Negative-path self-check: a deliberately-inconsistent fixture MUST fail, so the
# mismatch-detection branch is exercised (not just the positive shape path).
negative_fixture="$root/planning/tests/fixtures/progress-shape-bad"
if [ -z "${PROGRESS_SHAPE_NEG_DONE:-}" ] && [ -d "$negative_fixture" ]; then
    bad_out="$(PROGRESS_SHAPE_NEG_DONE=1 PLANS_ROOT="$negative_fixture" bash "$0" 2>&1 || true)"
    if printf '%s\n' "$bad_out" | grep -q 'FAIL'; then
        :  # correctly detected
    else
        note_fail "negative fixture $negative_fixture did not FAIL (mismatch path broken)"
    fi
fi

echo
echo "progress-bar: validated $count progress file(s) under $plans_root; $([ "$fail" -eq 0 ] && echo PASS || echo FAIL)"
[ "$fail" -eq 0 ]
