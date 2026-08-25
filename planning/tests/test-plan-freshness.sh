#!/usr/bin/env bash
# MODE: DEV
# test-plan-freshness.sh — the propagation freshness pass names units whose
# target code was committed after the plan's own last commit, and stays silent
# once the plan record catches up (B38/T44).
#
# Unit harness: plan-map-lib and the propagation lib are sourced directly over
# a throwaway git repository, with hand-built maps and pinned commit dates, so
# the ordering under test is data rather than clock luck.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "planning/tests/lib-test.sh" 2>/dev/null || source "/home/tschallacka/git/ai-skills/planning/tests/lib-test.sh"
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scripts="$(cd "$tests_dir/../scripts" && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/plan-freshness.XXXXXX")"
trap 'rm -rf "$work"' EXIT

export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
T0=2026-08-01T00:00:00+0000
T1=2026-08-02T00:00:00+0000
T2=2026-08-03T00:00:00+0000

repo="$work/repo"
mkdir -p "$repo" "$repo/src" "$repo/plan/01-g"
git -C "$repo" init -q
git -C "$repo" checkout -q -b main

printf 'thing()\n' > "$repo/src/change-me.php"
printf '# Goal: G\n' > "$repo/plan/01-g/goal.md"

commit_at() { # <date> <message>
    git -C "$repo" add -A >/dev/null 2>&1 || true
    GIT_AUTHOR_DATE="$1" GIT_COMMITTER_DATE="$1" git -C "$repo" \
        commit -q -m "$2" >/dev/null 2>&1 || true
}

# shellcheck source=planning/scripts/plan-map-lib.sh
source "$scripts/plan-map-lib.sh"
# shellcheck source=planning/scripts/validate-plan-common-lib.sh
source "$scripts/validate-plan-common-lib.sh"
# shellcheck source=planning/scripts/validate-plan-propagation-lib.sh
source "$scripts/validate-plan-propagation-lib.sh"

# fail() deliberately shadows common-lib's error-counting fail: this harness
# reports through its own counter instead of a shared errors tally.
fail() { printf 'plan-freshness: %s\n' "$1" >&2; FAILED=1; }
FAILED=0

plan_dir="$repo/plan"
repo_root="$repo"

plan_map_set unit_type W01 source
plan_map_set unit_file W01 src/change-me.php

run_freshness() { # -> warnings on stdout
    local out
    out="$(mktemp "${TMPDIR:-/tmp}/plan-freshness-out.XXXXXX")"
    plan_validate_propagation_freshness 2> "$out" || true
    cat "$out"
    rm -f "$out"
}

commit_at "$T0" "the plan, as written"

# ---- control first: nothing has moved since the plan record -----------------
out="$(run_freshness)"
case "$out" in
    *'changed at'*) fail "control run reported drift where none exists: $out" ;;
esac

# ---- code moves after the plan: the B38 signature must be named -------------
printf 'thing()\n# touched by the feature\n' > "$repo/src/change-me.php"
commit_at "$T1" "the code lands after the plan"
out="$(run_freshness)"
case "$out" in
    *'unit W01 target'*'changed at'*) : ;;
    *) fail "drift after the last plan record was not named for W01: [$out]" ;;
esac

# ---- the plan catching up clears it -----------------------------------------
printf '\nRecorded: the change shipped.\n' >> "$repo/plan/01-g/goal.md"
commit_at "$T2" "the plan records the mutation"
out="$(run_freshness)"
case "$out" in
    *'changed at'*) fail "drift still reported after the plan caught up: $out" ;;
esac

# ---- a unit outside the repository is skipped without noise ------------------
plan_map_set unit_type W02 source
plan_map_set unit_file W02 ../elsewhere/x.php
out="$(run_freshness)"
case "$out" in
    *W02*) fail "a target outside the repository was treated as drift: $out" ;;
esac

[ "$FAILED" -eq 0 ] || exit 1
printf '%s\n' 'test-plan-freshness: PASS'
