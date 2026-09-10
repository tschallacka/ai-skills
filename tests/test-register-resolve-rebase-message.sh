#!/usr/bin/env bash
# MODE: DEV
# COVERS: src/bug-report/src/resolve.rs src/bug-report/src/main.rs
# test-register-resolve-rebase-message.sh — B151: `bugs resolve`/`todo resolve`
# name a real action ("fix that side on its own branch first") only when there
# is a branch to fix. Under `git rebase`, neither index side is a branch tip --
# `ours` is HEAD mid-replay, `theirs` is the commit currently being applied --
# so that instruction has no referent. This drives a REAL git rebase into a
# real conflict where one side's register is unsound on its own, and asserts
# the refusal names a rebase-appropriate action instead; a second case drives a
# REAL git merge the same way and asserts the ORIGINAL branch-oriented wording
# is unchanged there, so the fix cannot be "always say the new thing".
#
# A missing cargo and no prebuilt bin/ binaries is a loud SKIP, not a failure
# (mirrors tests/test-register-file-flags.sh, the sibling test this one
# borrows its build/skip structure from).
#
# Usage: test-register-resolve-rebase-message.sh
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$root/planning/tests/lib-test.sh"
t_begin

export LC_ALL=C
work="$(mktemp -d "${TMPDIR:-/tmp}/register-resolve-rebase.XXXXXX")"
trap 'rm -rf "$work"' EXIT

BUGS="$root/target/release/bugs"
if ! command -v cargo >/dev/null 2>&1; then
    prebuilt_bugs="$(ls "$root"/bug-report/bin/*/bugs 2>/dev/null | head -1 || true)"
    if [ -n "$prebuilt_bugs" ]; then
        BUGS="$prebuilt_bugs"
    else
        t_skip 'test-register-resolve-rebase-message: no cargo and no prebuilt bin/ binaries - rust assertions did not run'
    fi
else
    ( cd "$root/src/bug-report" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build bug-report failed"
fi

git_repo() { # <dir>
    git init -q -b trunk "$1"
    git -C "$1" config user.email test@example.com
    git -C "$1" config user.name "Test"
}

base_bugs_json() {
    cat <<'JSON'
{
  "skill": "bug-report",
  "skill_version": "2.0.0-alpha.1",
  "comment": "fixture",
  "bugs": [
    {
      "id": "B1",
      "title": "First bug",
      "status": "confirmed",
      "severity": "minor",
      "priority": "normal",
      "parent": null,
      "reproduce": "r",
      "observed": "o",
      "expected": "e",
      "mechanism": "m",
      "surfaces": [],
      "fix": null,
      "verification": null,
      "found_by": "",
      "notes": null,
      "created_at": "2026-01-01T00:00:00Z",
      "updated_at": "2026-01-01T00:00:00Z"
    }
  ]
}
JSON
}

# The unsound side: a second bug that collided on B1 (as an independent branch
# adding a bug at the same next-free id would), taken alone with no later
# commit on this same side to renumber it away.
unsound_bugs_json() {
    cat <<'JSON'
{ "skill": "bug-report", "skill_version": "2.0.0-alpha.1",
  "comment": "fixture, mid-history", "bugs": [
    { "id": "B1", "title": "First bug", "status": "confirmed",
      "severity": "minor", "priority": "normal", "parent": null,
      "reproduce": "r", "observed": "o", "expected": "e", "mechanism": "m",
      "surfaces": [], "fix": null, "verification": null, "found_by": "",
      "notes": null, "created_at": "2026-01-01T00:00:00Z",
      "updated_at": "2026-01-01T00:00:00Z" },
    { "id": "B1", "title": "Second bug, same id", "status": "confirmed",
      "severity": "minor", "priority": "normal", "parent": null,
      "reproduce": "r2", "observed": "o2", "expected": "e2", "mechanism": "m2",
      "surfaces": [], "fix": null, "verification": null, "found_by": "",
      "notes": null, "created_at": "2026-01-02T00:00:00Z",
      "updated_at": "2026-01-02T00:00:00Z" }
  ] }
JSON
}

# The other side: a real, conflicting edit to the SAME field of BUGS.json, so
# replaying the unsound commit lands a genuine textual conflict rather than a
# clean fast-forward or an auto-merge.
trunk_bugs_json() {
    cat <<'JSON'
{
  "skill": "bug-report",
  "skill_version": "2.0.0-alpha.1",
  "comment": "fixture, trunk moved on",
  "bugs": [
    {
      "id": "B1",
      "title": "First bug",
      "status": "confirmed",
      "severity": "minor",
      "priority": "normal",
      "parent": null,
      "reproduce": "r",
      "observed": "o",
      "expected": "e",
      "mechanism": "m",
      "surfaces": [],
      "fix": null,
      "verification": null,
      "found_by": "",
      "notes": null,
      "created_at": "2026-01-01T00:00:00Z",
      "updated_at": "2026-01-01T00:00:00Z"
    }
  ]
}
JSON
}

# ── the rebase case: the new, rebase-appropriate refusal ────────────────────
repo="$work/rebase-repo"
git_repo "$repo"
base_bugs_json > "$repo/BUGS.json"
git -C "$repo" add BUGS.json
git -C "$repo" commit -q -m base

git -C "$repo" checkout -q -b topic
unsound_bugs_json > "$repo/BUGS.json"
git -C "$repo" add BUGS.json
git -C "$repo" commit -q -m "topic: duplicate id, to be renumbered later"

git -C "$repo" checkout -q trunk
trunk_bugs_json > "$repo/BUGS.json"
git -C "$repo" add BUGS.json
git -C "$repo" commit -q -m "trunk: moved on"

git -C "$repo" checkout -q topic
if git -C "$repo" rebase trunk >/dev/null 2>&1; then
    t_fail "the rebase fixture did not conflict; the scenario setup is wrong"
else
    rebase_err="$(cd "$repo" && "$BUGS" resolve --file BUGS.json 2>&1 >/dev/null)"
    case "$rebase_err" in
        *'conflicted mid-rebase'*) ;;
        *) t_fail "rebase: the refusal did not name the rebase case: $rebase_err" ;;
    esac
    case "$rebase_err" in
        *'fix that side on its own branch first'*)
            t_fail "rebase: the refusal still printed the merge-only wording: $rebase_err" ;;
    esac
    case "$rebase_err" in
        *'no branch to go'*'fix'*) ;;
        *) t_fail "rebase: the refusal did not say there is no branch to fix: $rebase_err" ;;
    esac
fi
git -C "$repo" rebase --abort >/dev/null 2>&1 || true

# ── the merge case: unchanged, still the original branch-oriented wording ──
repo2="$work/merge-repo"
git_repo "$repo2"
base_bugs_json > "$repo2/BUGS.json"
git -C "$repo2" add BUGS.json
git -C "$repo2" commit -q -m base

git -C "$repo2" checkout -q -b other
unsound_bugs_json > "$repo2/BUGS.json"
git -C "$repo2" add BUGS.json
git -C "$repo2" commit -q -m "other: duplicate id"

git -C "$repo2" checkout -q trunk
trunk_bugs_json > "$repo2/BUGS.json"
git -C "$repo2" add BUGS.json
git -C "$repo2" commit -q -m "trunk: moved on"

if git -C "$repo2" merge other >/dev/null 2>&1; then
    t_fail "the merge fixture did not conflict; the scenario setup is wrong"
else
    merge_err="$(cd "$repo2" && "$BUGS" resolve --file BUGS.json 2>&1 >/dev/null)"
    case "$merge_err" in
        *'fix that side on its own branch first'*) ;;
        *) t_fail "merge: the original branch-oriented refusal regressed: $merge_err" ;;
    esac
    case "$merge_err" in
        *'conflicted mid-rebase'*)
            t_fail "merge: the rebase-only wording leaked into a real merge: $merge_err" ;;
    esac
fi
git -C "$repo2" merge --abort >/dev/null 2>&1 || true

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-register-resolve-rebase-message: PASS'
