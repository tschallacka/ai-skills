#!/usr/bin/env bash
# MODE: PROD
# plans-board-lib.sh — measure one plans root for the cross-plan board.
#
# Owns two jobs the board script does not: finding every plan under a root,
# and reducing one plan directory to the handful of numbers a board row shows.
# It reads the plan tree directly rather than calling overview-state.sh, which
# costs 8-9 seconds per plan and would make a 25-plan root take minutes.
#
# Usage:
#   source "$script_dir/plans-board-lib.sh"     # sourced, never executed
#
# Every function is prefixed `board_` so nothing shadows a caller's name.

# ─────────────────────────────────────────────────────────────────────────────
# Discovery — what counts as a plan, and what is merely a directory
# ─────────────────────────────────────────────────────────────────────────────

# board_find_plans ROOT — print each plan directory under ROOT, one per line.
#
# A plan is a directory holding plan-description.md. Depth matters: plan-root.sh
# scopes a global root as <root>/<owner>/<repo>, so plans live at depth 1 or
# depth 3 below ROOT depending on how the project was set up. A top-level glob
# would find the first and silently miss the second.
board_find_plans() {
    [ -n "${1:-}" ] || { printf 'board_find_plans: root required\n' >&2; return 1; }
    [ -d "$1" ] || return 0
    # -maxdepth 5 covers <root>/<owner>/<sub>/<repo>/plan-description.md.
    find "$1" -maxdepth 5 -name plan-description.md -print 2>/dev/null \
        | sed 's#/plan-description\.md$##' \
        | LC_ALL=C sort
}

# board_find_strays ROOT — print directories under ROOT that hold no plan and
# contain no plan either, one per line.
#
# These exist: a plans root accumulates directories from older planning-skill
# formats (a flat set of NN-step-*.md files) and the occasional empty one. A
# board that lists only the plans it understands and calls itself the plans root
# is lying by omission, so they are named rather than dropped.
board_find_strays() {
    [ -n "${1:-}" ] || { printf 'board_find_strays: root required\n' >&2; return 1; }
    [ -d "$1" ] || return 0
    local dir
    while IFS= read -r dir; do
        [ -n "$dir" ] || continue
        [ -f "$dir/plan-description.md" ] && continue
        # A parent of a nested plan is scaffolding, not a stray.
        # PORTABILITY(pipefail-grep-q): the emptiness test is a command
        # substitution rather than `find | head | grep -q`, because under
        # pipefail a `find` killed by head's SIGPIPE reports 141 — which is the
        # case where a plan WAS found, so the pipeline form inverts the answer.
        [ -z "$(find "$dir" -maxdepth 4 -name plan-description.md -print 2>/dev/null | head -1)" ] \
            || continue
        printf '%s\n' "$dir"
    done < <(find "$1" -maxdepth 1 -mindepth 1 -type d -print 2>/dev/null | LC_ALL=C sort)
}

# ─────────────────────────────────────────────────────────────────────────────
# Measurement — one plan directory reduced to counters
# ─────────────────────────────────────────────────────────────────────────────

# board_skipped_goals PLAN_DIR — the names of goals the plan-level progress
# table marks skipped, one per line.
#
# A skip is recorded at goal level and does not propagate down: measured on
# xcore-paymentmethod-code-aliases, goal 09 reads `⏭️ skipped` in the plan
# progress table while all six of its own step rows still read `💤 incomplete`.
# Counting only the step rows therefore reports that plan as 76 of 82 and
# stalled, when its own overall line reads 100% complete.
board_skipped_goals() {
    local plan="${1:-}" line name
    [ -n "$plan" ] || { printf 'board_skipped_goals: plan directory required\n' >&2; return 1; }
    [ -f "$plan/progress.md" ] || return 0
    while IFS= read -r line; do
        case "$line" in '| '*'|') ;; *) continue ;; esac
        case "$line" in *'⏭'*) ;; *) continue ;; esac
        name="$(plan_table_cell "$line" 2)"
        [ -n "$name" ] && printf '%s\n' "$name"
    done < "$plan/progress.md"
}

# board_step_counts PLAN_DIR — print "<done> <skipped> <wip> <total>".
#
# Counted from the per-goal progress tables, which are the rows that carry a
# step, with every step of a goal the plan marks skipped counted as skipped.
# Skipped is counted apart from done because a plan whose remaining steps are
# all skipped is finished, while one with the same done-count and open steps is
# not — collapsing the two reports a delivered plan as stalled.
board_step_counts() {
    local plan="${1:-}" gf line goal skipped done_n=0 skip_n=0 wip_n=0 all_n=0
    [ -n "$plan" ] || { printf 'board_step_counts: plan directory required\n' >&2; return 1; }
    skipped="$(board_skipped_goals "$plan")"
    for gf in "$plan"/*/progress.md; do
        [ -f "$gf" ] || continue
        goal="$(basename "$(dirname "$gf")")"
        while IFS= read -r line; do
            case "$line" in '| '*'|') ;; *) continue ;; esac
            case "$line" in *'---'*|*'| Stepname |'*|*'| Goalname |'*) continue ;; esac
            all_n=$((all_n + 1))
            case "$(printf '\n%s\n' "$skipped")" in
                *"$(printf '\n%s\n' "$goal")"*) skip_n=$((skip_n + 1)); continue ;;
            esac
            case "$line" in
                *'✅'*) done_n=$((done_n + 1)) ;;
                *'⏭'*) skip_n=$((skip_n + 1)) ;;
                *'⏳'*) wip_n=$((wip_n + 1)) ;;
            esac
        done < "$gf"
    done
    printf '%s %s %s %s\n' "$done_n" "$skip_n" "$wip_n" "$all_n"
}

# board_finding_counts PLAN_DIR — print "<open> <resolved>" from the live
# adversarial-review findings table. A row whose status carries ✅ is resolved;
# anything else is still open, including a row with no status at all.
board_finding_counts() {
    local plan="${1:-}" rev line open_n=0 res_n=0
    [ -n "$plan" ] || { printf 'board_finding_counts: plan directory required\n' >&2; return 1; }
    rev="$plan/adversarial-review.md"
    if [ -f "$rev" ]; then
        while IFS= read -r line; do
            case "$line" in '| AR-'*) ;; *) continue ;; esac
            case "$line" in *'No finding recorded yet'*) continue ;; esac
            case "$line" in
                *'✅'*) res_n=$((res_n + 1)) ;;
                *) open_n=$((open_n + 1)) ;;
            esac
        done < "$rev"
    fi
    printf '%s %s\n' "$open_n" "$res_n"
}

# board_review_status PLAN_DIR — the review verdict as one bare word:
# approved, pending, or none when the plan has no review document at all.
board_review_status() {
    local plan="${1:-}" rev raw
    [ -n "$plan" ] || { printf 'board_review_status: plan directory required\n' >&2; return 1; }
    rev="$plan/adversarial-review.md"
    [ -f "$rev" ] || { printf 'none\n'; return 0; }
    raw="$(sed -n 's/.*- Status:[[:space:]]*`//p' "$rev" | head -1)"
    case "$raw" in
        *approved*) printf 'approved\n' ;;
        '') printf 'none\n' ;;
        *) printf 'pending\n' ;;
    esac
}

# board_lifecycle REVIEW DONE SKIPPED TOTAL — the state the board sorts and
# colours by.
#
# planning       the review has not approved yet, so the question is whether
#                the plan is sound. Zero progress is correct here.
# awaiting-work  approved, but the plan has no steps to execute at all. This is
#                its own state rather than implementing: a plan with nothing
#                decomposed is not executing, and calling it implementing would
#                put a card claiming work in progress next to plans that have it.
# implementing   approved and executing.
# complete       approved and every step is either done or deliberately skipped.
board_lifecycle() {
    local review="${1:-none}" done_n="${2:-0}" skip_n="${3:-0}" all_n="${4:-0}"
    if [ "$review" != approved ]; then
        printf 'planning\n'
        return 0
    fi
    if [ "$all_n" -le 0 ]; then
        printf 'awaiting-work\n'
    elif [ "$((done_n + skip_n))" -ge "$all_n" ]; then
        printf 'complete\n'
    else
        printf 'implementing\n'
    fi
}

# board_last_activity PLAN_DIR — seconds since the most recently modified
# markdown file in the plan, or the empty string when it cannot be told.
#
# No `stat -c` and no `find -printf`: both are GNU-only. `find -newer` against a
# ladder of reference files needs no format string and works on either userland.
board_last_activity() {
    local plan="${1:-}" ref age
    [ -n "$plan" ] || { printf 'board_last_activity: plan directory required\n' >&2; return 1; }
    ref="$(mktemp "${TMPDIR:-/tmp}/board-age.XXXXXX")"
    for age in 3600 86400 604800 2592000 7776000; do
        # Backdate the reference by touching it, then comparing. touch -t needs a
        # timestamp; -r plus a second file is simpler and portable.
        # PORTABILITY(pipefail-grep-q): see board_find_strays — the same
        # inverted-answer hazard, so the same command-substitution form.
        if board_touch_ago "$ref" "$age" \
            && [ -n "$(find "$plan" -name '*.md' -newer "$ref" -print 2>/dev/null | head -1)" ]; then
            rm -f "$ref"
            printf '%s\n' "$age"
            return 0
        fi
    done
    rm -f "$ref"
    printf '\n'
}

# board_touch_ago FILE SECONDS — set FILE's mtime SECONDS into the past.
# `date -d` is GNU and `date -v` is BSD, so try both and give up honestly.
board_touch_ago() {
    local file="${1:-}" secs="${2:-0}" stamp
    stamp="$(date -u -d "@$(( $(date -u +%s) - secs ))" '+%Y%m%d%H%M.%S' 2>/dev/null \
        || date -u -r "$(( $(date -u +%s) - secs ))" '+%Y%m%d%H%M.%S' 2>/dev/null || true)"
    [ -n "$stamp" ] || return 1
    touch -t "$stamp" "$file" 2>/dev/null
}

# board_ago_label SECONDS — the age bucket as text a reader can scan.
board_ago_label() {
    case "${1:-}" in
        '')      printf 'over 90 days\n' ;;
        3600)    printf 'within the hour\n' ;;
        86400)   printf 'today\n' ;;
        604800)  printf 'this week\n' ;;
        2592000) printf 'this month\n' ;;
        *)       printf 'this quarter\n' ;;
    esac
}
