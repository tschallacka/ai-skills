#!/usr/bin/env bash
# MODE: DEV
# test-plan-snapshot.sh — the pre-mutation git snapshot fires in every layout
# whose repository the skill owns, and stays out of the user's own repository.
#
# The bug this pins: plan_git_snapshot used to guard on `$plan_dir/.git`, but
# create-plan.sh puts the repository at the plans root in the common case, so
# every mutating helper no-oped and an overwritten paragraph was unrecoverable.
# Only the "explicit path outside any repo" layout worked, and that was the one
# layout the suite exercised.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scripts_dir="$(cd "$tests_dir/../scripts" && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/plan-snapshot-test.XXXXXX")"
trap 'rm -rf "$work"' EXIT

note_fail() {
    printf 'FAIL: %s\n' "$1" >&2
    t_record "$1"
}

git_here() {
    git -C "$1" -c user.name=t -c user.email=t@t "${@:2}"
}

# Create a plan, mutate the same paragraph twice, and report what git retained.
# The first mutation dirties the tree; the second must snapshot it.
probe() {
    local root="$1" name="$2"
    local plan_dir="$root/$name"
    PLANS_ROOT="$root" PLAN_NONINTERACTIVE=1 \
        "$scripts_dir/create-plan.sh" "$name" 'Snapshot probe' >/dev/null 2>&1 || true
    [ -d "$plan_dir" ] || { printf 'NOPLAN\n'; return 0; }
    "$scripts_dir/update-plan-content.sh" -ds "$plan_dir" current-state \
        -p '2.1: first mutation' >/dev/null 2>&1 || true
    "$scripts_dir/update-plan-content.sh" -ds "$plan_dir" current-state \
        -p '2.1: second mutation' >/dev/null 2>&1 || true
    local landed=0 retained=0 history
    grep -Fq 'second mutation' "$plan_dir/plan-description.md" && landed=1
    # Captured, not piped into grep -q: `grep -q` exits on the first match,
    # SIGPIPEs git, and pipefail then reports 141 for a pipeline that matched.
    history="$(git -C "$plan_dir" log -p --all 2>/dev/null || true)"
    case "$history" in
        *'first mutation'*) retained=1 ;;
    esac
    printf '%s %s\n' "$landed" "$retained"
}

# The manifest stores values through printf %q, so an empty pin is the two
# characters '' and a path with a space is escaped. Undo that before comparing.
pinned_repo() {
    local assignment
    assignment="$(grep '^PLAN_SNAPSHOT_REPO=' "$1/.env")" || return 0
    eval "printf '%s' ${assignment#PLAN_SNAPSHOT_REPO=}"
}

# ---- Layout A: a project's .plans, gitignored — the documented common case ----
mkdir -p "$work/projA"
git init -q "$work/projA"
printf '/.plans\n' > "$work/projA/.gitignore"
git_here "$work/projA" add -A
git_here "$work/projA" commit -qm init
read -r landed retained <<<"$(probe "$work/projA/.plans" plan-a)"
[ "$landed" = 1 ] || note_fail 'layout A: the mutation itself did not land'
[ "$retained" = 1 ] || note_fail 'layout A: overwritten text was not snapshotted into the plans-root repo'
[ -n "$(pinned_repo "$work/projA/.plans/plan-a")" ] \
    || note_fail 'layout A: PLAN_SNAPSHOT_REPO was left empty'

# A second plan under the same plans root must snapshot too: the repository
# already exists, so create-plan.sh takes the "tracked in $top" branch, and
# $top is the plans root.
read -r landed retained <<<"$(probe "$work/projA/.plans" plan-a2)"
[ "$retained" = 1 ] || note_fail 'layout A: the second plan in a shared plans root did not snapshot'

# ---- Layout B: plans root outside any repository ----
mkdir -p "$work/rootB"
read -r landed retained <<<"$(probe "$work/rootB" plan-b)"
[ "$retained" = 1 ] || note_fail 'layout B: overwritten text was not snapshotted'

# ---- Layout C: plans tracked in the user's own repository ----
# The pin must be empty and no snapshot may be written: a commit per mutation
# in somebody's project history is worse than the lost undo.
mkdir -p "$work/projC"
git init -q "$work/projC"
git_here "$work/projC" commit -q --allow-empty -m init
read -r landed retained <<<"$(probe "$work/projC/plans" plan-c)"
[ "$landed" = 1 ] || note_fail 'layout C: the mutation itself did not land'
[ "$retained" = 0 ] || note_fail "layout C: snapshotted into the user's own repository"
[ -z "$(pinned_repo "$work/projC/plans/plan-c")" ] \
    || note_fail 'layout C: PLAN_SNAPSHOT_REPO must be empty for a user-owned repository'
before="$(git -C "$work/projC" rev-list --count HEAD)"
[ "$before" = 2 ] \
    || note_fail "layout C: expected only the initial plan commit in the user's repo, found $before"

# ---- Layout D: an explicit path outside any repository ----
mkdir -p "$work/outsideD"
read -r landed retained <<<"$(probe "$work/outsideD/sub" plan-d)"
[ "$retained" = 1 ] || note_fail 'layout D: overwritten text was not snapshotted'

# ---- A relative plan path must still snapshot ----
# The add runs with git's cwd set to the snapshot repository, so a caller's
# relative path is read against the repository rather than the caller's cwd.
# Exercised from the project root, where the two differ.
rel_landed=0
rel_retained=0
(
    cd "$work/projA" || exit 1
    "$scripts_dir/update-plan-content.sh" -ds .plans/plan-a current-state \
        -p '2.1: relative first' >/dev/null 2>&1 || true
    "$scripts_dir/update-plan-content.sh" -ds .plans/plan-a current-state \
        -p '2.1: relative second' >/dev/null 2>&1 || true
)
grep -Fq 'relative second' "$work/projA/.plans/plan-a/plan-description.md" && rel_landed=1
rel_history="$(git -C "$work/projA/.plans" log -p --all 2>/dev/null || true)"
case "$rel_history" in
    *'relative first'*) rel_retained=1 ;;
esac
[ "$rel_landed" = 1 ] || note_fail 'relative path: the mutation itself did not land'
[ "$rel_retained" = 1 ] \
    || note_fail 'relative path: overwritten text was not snapshotted (pathspec resolved against the repo, not the caller)'

# ---- Regenerating the manifest must not lose the pin ----
plan_a="$work/projA/.plans/plan-a"
want="$(pinned_repo "$plan_a")"
"$scripts_dir/plan-env.sh" write-plan "$plan_a" "$work/projA/.plans" >/dev/null
[ "$(pinned_repo "$plan_a")" = "$want" ] \
    || note_fail 'plan-env.sh write-plan without an argument erased PLAN_SNAPSHOT_REPO'
"$scripts_dir/plan-env.sh" check "$plan_a" "$work/projA/.plans" >/dev/null \
    || note_fail 'plan-env.sh check refused a manifest it had just written'

# ---- A pin outside the plans root or the plan is refused ----
rc=0
"$scripts_dir/plan-env.sh" write-plan "$plan_a" "$work/projA/.plans" "$work/projC" >/dev/null 2>&1 || rc=$?
[ "$rc" = 64 ] || note_fail "an out-of-tree snapshot repo was accepted (exit $rc, want 64)"

if [ "$(t_failures)" -ne 0 ]; then
    printf 'test-plan-snapshot: %d failure(s).\n' "$(t_failures)" >&2
    exit 1
fi
printf 'test-plan-snapshot: PASS\n'
