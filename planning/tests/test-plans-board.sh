#!/usr/bin/env bash
# MODE: DEV
# test-plans-board.sh — the cross-plan board renderer.
#
# Pins the readings that are easy to get wrong and impossible to notice from the
# page: a nested plan is discovered, a goal-level skip counts as skipped rather
# than as unfinished work, an approved plan with no steps is not called
# implementing, and the directories that are not plans are named rather than
# quietly dropped. Also pins self-containment, determinism and the refusals.
#
# Every fixture is built here. The live plans root is user data and a
# plans-root helper can delete it, so a test that read it would be neither
# reproducible nor safe.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
NOW=2026-08-28T00:00Z

root="$(mktemp -d "${TMPDIR:-/tmp}/plans-board.XXXXXX")"
trap 'rm -rf "$root"' EXIT

# ─────────────────────────────────────────────────────────────────────────────
# Fixtures — four plans covering the four lifecycles, plus the two shapes a
# real plans root carries that are not plans at all.
# ─────────────────────────────────────────────────────────────────────────────

seed_plan() { # <plan-dir> <title>
    mkdir -p "$1"
    printf '# Plan: %s\n\n## Current state\n\nx\n\n## Desired outcome\n\ny\n' "$2" \
        > "$1/plan-description.md"
    printf '# Progress: %s\n\n| Goalname | Description | Completion status |\n|---|---|---|\n' \
        "$(basename "$1")" > "$1/progress.md"
}

seed_goal() { # <plan-dir> <goal> <goal-status> <step-status>...
    local plan="$1" goal="$2" gstatus="$3" n=0 s
    shift 3
    mkdir -p "$plan/$goal/steps"
    printf '# Goal\n\n## Outcome and definition of done\n\nx\n' > "$plan/$goal/goal.md"
    printf '| %s | a goal | %s |\n' "$goal" "$gstatus" >> "$plan/progress.md"
    printf '# Progress: %s\n\n| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n' \
        "$goal" > "$plan/$goal/progress.md"
    for s in "$@"; do
        n=$((n + 1))
        printf '| %s | %02d-step-x | a step | %s |\n' "$goal" "$n" "$s" >> "$plan/$goal/progress.md"
    done
}

seed_review() { # <plan-dir> <status-text>
    printf '# Adversarial review\n\n- Status: `%s`\n\n## Findings\n\n' "$2" > "$1/adversarial-review.md"
    printf '| ID | Item | Change | Status | Work unit |\n|---|---|---|---|---|\n' >> "$1/adversarial-review.md"
    printf '| AR-01 | a thing | fix it | %s | W01 |\n\n## Verdict\n\nx\n' "$3" >> "$1/adversarial-review.md"
}

# Implementing: approved, half the steps done.
seed_plan "$root/alpha-running" 'Alpha running'
seed_goal "$root/alpha-running" 01-first '⏳ in progress' '✅ completed' '💤 incomplete'
seed_review "$root/alpha-running" '✅ approved' '💤 open'

# Complete through a goal-level skip: goal 02 is marked skipped in the PLAN
# table while its own step rows still read incomplete, which is how a real plan
# records a skip (measured on xcore-paymentmethod-code-aliases).
seed_plan "$root/beta-skipped" 'Beta with a skipped goal'
seed_goal "$root/beta-skipped" 01-first '✅ completed' '✅ completed' '✅ completed'
seed_goal "$root/beta-skipped" 02-optional '⏭️ skipped' '💤 incomplete' '💤 incomplete'
seed_review "$root/beta-skipped" '✅ approved' '✅ resolved'

# Approved with no steps at all.
seed_plan "$root/gamma-empty" 'Gamma with nothing decomposed'
seed_review "$root/gamma-empty" '✅ approved' '✅ resolved'

# Planning, and nested one level down the way a global root scopes by project.
seed_plan "$root/someowner/delta-nested" 'Delta nested under an owner'
seed_goal "$root/someowner/delta-nested" 01-first '💤 incomplete' '💤 incomplete'
seed_review "$root/someowner/delta-nested" '💤 pending' '💤 open'

# A hostile title. Plan text is read from a file, so it is attacker-adjacent:
# it must reach the page escaped, and it must never be expanded on the way. The
# row travels through a read, so a heredoc or here-string that re-scanned its
# own expansion would run the command substitution.
hostile='Title with <script>alert(1)</script> & "quotes" and $(id) and `id`'
seed_plan "$root/epsilon-hostile" "$hostile"
seed_goal "$root/epsilon-hostile" 01-first '💤 incomplete' '💤 incomplete'

# Not plans: an older flat format, and an empty directory.
mkdir -p "$root/legacy-flat" "$root/quite-empty"
printf '# a step\n' > "$root/legacy-flat/01-step-something.md"

out="$root/board.html"
BOARD_NOW=$NOW "$script_dir/render-plans-board.sh" --root "$root" --out "$out" >/dev/null
page="$(cat "$out")"

life_of() { # <plan-dir-name>  → the lifecycle the card carries
    tr '<' '\n<' < "$out" \
        | awk -v p="$1" '
            /article class="card" data-life=/ { life = $0; sub(/.*data-life="/, "", life); sub(/".*/, "", life) }
            $0 ~ ("p class=\"path mono\">" p "$") { print life; exit }'
}

# ─────────────────────────────────────────────────────────────────────────────
# Discovery
# ─────────────────────────────────────────────────────────────────────────────

# A nested plan is found. Mutation that breaks it: lower board_find_plans's
# -maxdepth to 1, and delta-nested disappears from the page entirely.
t_assert_contains 'nested plan discovered' 'someowner/delta-nested' "$page"

t_assert_eq 'plans counted' \
    "$(tr '<' '\n<' < "$out" | grep -c 'article class="card"')" 5

# ─────────────────────────────────────────────────────────────────────────────
# Hostile plan text
# ─────────────────────────────────────────────────────────────────────────────

# Mutation: replace esc() with `printf '%s' "$1"` and the raw script tag lands
# in the page.
t_assert_contains 'markup in a title is escaped' \
    '&lt;script&gt;alert(1)&lt;/script&gt;' "$page"
case "$page" in
    *'<script>alert(1)'*) t_fail 'a title reached the page as live markup' ;;
esac

# The row travels through a read; nothing in it may be expanded. `id` prints a
# uid= line, so its appearance anywhere on the page means a substitution ran.
t_assert_contains 'command substitution left literal' 'and $(id) and' "$page"
case "$page" in
    *uid=*) t_fail 'a command substitution in a plan title was executed' ;;
esac

# The directories that are not plans are named. Mutation: make
# board_find_strays print nothing, and the page silently shows 4 of 6.
t_assert_contains 'legacy directory named' 'legacy-flat' "$page"
t_assert_contains 'empty directory named' 'quite-empty' "$page"
t_assert_contains 'strays explained, not scored' 'no plan-description.md' "$page"

# ─────────────────────────────────────────────────────────────────────────────
# Lifecycle readings
# ─────────────────────────────────────────────────────────────────────────────

t_assert_eq 'approved with work left is implementing' "$(life_of alpha-running)" implementing

# Mutation: drop the board_skipped_goals consultation from board_step_counts
# and beta-skipped reads implementing at 2 of 4, contradicting its own plan
# table, which marks goal 02 skipped.
t_assert_eq 'goal-level skip completes the plan' "$(life_of beta-skipped)" complete

# Mutation: remove the all_n <= 0 arm of board_lifecycle and gamma-empty is
# called implementing, claiming work in progress where there are no steps.
t_assert_eq 'approved with no steps is not implementing' "$(life_of gamma-empty)" awaiting-work

t_assert_eq 'unapproved is planning' "$(life_of someowner/delta-nested)" planning

t_assert_contains 'zero progress in planning is stated as correct' \
    'Zero progress is the correct reading' "$page"

# ─────────────────────────────────────────────────────────────────────────────
# The page itself
# ─────────────────────────────────────────────────────────────────────────────

# Self-contained: no external asset may be referenced. The only href is a
# relative link into a plan's own overview.
# PORTABILITY(pipefail-grep-q): the match is captured rather than tested through
# a pipeline, so a SIGPIPE'd producer cannot be read as "no match" under
# pipefail.
external="$(tr '<' '\n<' < "$out" | grep -E 'src=|href="(https?:)?//|url\(' || true)"
[ -z "$external" ] || t_fail "page references an external asset: $external"

t_assert_contains 'skip is shown apart from done' 'skipped' "$page"
t_assert_contains 'timestamp is the pinned one' "$NOW" "$page"

# Deterministic: two renders of an unchanged root are byte-identical.
second="$root/board-2.html"
BOARD_NOW=$NOW "$script_dir/render-plans-board.sh" --root "$root" --out "$second" >/dev/null
cmp -s "$out" "$second" || t_fail 'two renders of one root differ'

# A plan with no overview.html says so rather than linking to a missing file.
t_assert_contains 'missing overview is stated' 'no overview rendered yet' "$page"
: > "$root/alpha-running/overview.html"
BOARD_NOW=$NOW "$script_dir/render-plans-board.sh" --root "$root" --out "$second" >/dev/null
t_assert_contains 'existing overview is linked' 'href="alpha-running/overview.html"' "$(cat "$second")"

# ─────────────────────────────────────────────────────────────────────────────
# Refusals
# ─────────────────────────────────────────────────────────────────────────────

t_expect_exit 66 'missing plans root refused' \
    "$script_dir/render-plans-board.sh" --root "$root/does-not-exist" --out "$second"
t_expect_exit 64 'unknown flag refused' \
    "$script_dir/render-plans-board.sh" --root "$root" --nonsense
t_expect_exit 64 'non-numeric refresh refused' \
    "$script_dir/render-plans-board.sh" --root "$root" --refresh soon
t_expect_exit 0 'help succeeds' "$script_dir/render-plans-board.sh" --help

t_end
