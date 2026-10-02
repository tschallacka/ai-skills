#!/usr/bin/env bash
# MODE: DEV
# COVERS: src/todo/src/resolve.rs src/todo/src/main.rs
# test-todo-resolve-rebase-message.sh — the todo half of B151, mirroring
# tests/test-register-resolve-rebase-message.sh's bugs case. `todo resolve`
# shares the same "fix that side on its own branch first" wording (and the
# same bug) as `bugs resolve`, in a separately-compiled crate with its own
# copy of resolve.rs, so it needs its own real-git-rebase proof rather than
# inheriting the bugs test's coverage.
#
# Usage: test-todo-resolve-rebase-message.sh
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$root/planning/tests/lib-test.sh"
t_begin

export LC_ALL=C
work="$(mktemp -d "${TMPDIR:-/tmp}/todo-resolve-rebase.XXXXXX")"
trap 'rm -rf "$work"' EXIT

TODO="$root/target/release/todo"
if ! command -v cargo >/dev/null 2>&1; then
    prebuilt_todo="$(ls "$root"/todo/bin/*/todo 2>/dev/null | head -1 || true)"
    if [ -n "$prebuilt_todo" ]; then
        TODO="$prebuilt_todo"
    else
        t_skip 'test-todo-resolve-rebase-message: no cargo and no prebuilt bin/ binaries - rust assertions did not run'
    fi
else
    ( cd "$root/src/todo" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build todo failed"
fi

git_repo() { # <dir>
    git init -q -b trunk "$1"
    git -C "$1" config user.email test@example.com
    git -C "$1" config user.name "Test"
}

base_todo_json() {
    cat <<'JSON'
{
  "skill": "todo",
  "skill_version": "2.0.0-alpha.1",
  "comment": "fixture",
  "tasks": [
    {
      "id": "T1",
      "title": "First task",
      "status": "open",
      "priority": "normal",
      "parent": null,
      "detail": "d",
      "blocked_on": null,
      "refs": [],
      "note": null,
      "created_at": "2026-01-01T00:00:00Z",
      "updated_at": "2026-01-01T00:00:00Z"
    }
  ]
}
JSON
}

unsound_todo_json() {
    cat <<'JSON'
{
  "skill": "todo",
  "skill_version": "2.0.0-alpha.1",
  "comment": "fixture, mid-history",
  "tasks": [
    {
      "id": "T1",
      "title": "First task",
      "status": "open",
      "priority": "normal",
      "parent": null,
      "detail": "d",
      "blocked_on": null,
      "refs": [],
      "note": null,
      "created_at": "2026-01-01T00:00:00Z",
      "updated_at": "2026-01-01T00:00:00Z"
    },
    {
      "id": "T1",
      "title": "Second task, same id",
      "status": "open",
      "priority": "normal",
      "parent": null,
      "detail": "d2",
      "blocked_on": null,
      "refs": [],
      "note": null,
      "created_at": "2026-01-02T00:00:00Z",
      "updated_at": "2026-01-02T00:00:00Z"
    }
  ]
}
JSON
}

trunk_todo_json() {
    cat <<'JSON'
{
  "skill": "todo",
  "skill_version": "2.0.0-alpha.1",
  "comment": "fixture, trunk moved on",
  "tasks": [
    {
      "id": "T1",
      "title": "First task",
      "status": "open",
      "priority": "normal",
      "parent": null,
      "detail": "d",
      "blocked_on": null,
      "refs": [],
      "note": null,
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
base_todo_json > "$repo/TODO.json"
git -C "$repo" add TODO.json
git -C "$repo" commit -q -m base

git -C "$repo" checkout -q -b topic
unsound_todo_json > "$repo/TODO.json"
git -C "$repo" add TODO.json
git -C "$repo" commit -q -m "topic: duplicate id, to be renumbered later"

git -C "$repo" checkout -q trunk
trunk_todo_json > "$repo/TODO.json"
git -C "$repo" add TODO.json
git -C "$repo" commit -q -m "trunk: moved on"

git -C "$repo" checkout -q topic
if git -C "$repo" rebase trunk >/dev/null 2>&1; then
    t_fail "the rebase fixture did not conflict; the scenario setup is wrong"
else
    rebase_err="$(cd "$repo" && "$TODO" resolve --file TODO.json 2>&1 >/dev/null)"
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
base_todo_json > "$repo2/TODO.json"
git -C "$repo2" add TODO.json
git -C "$repo2" commit -q -m base

git -C "$repo2" checkout -q -b other
unsound_todo_json > "$repo2/TODO.json"
git -C "$repo2" add TODO.json
git -C "$repo2" commit -q -m "other: duplicate id"

git -C "$repo2" checkout -q trunk
trunk_todo_json > "$repo2/TODO.json"
git -C "$repo2" add TODO.json
git -C "$repo2" commit -q -m "trunk: moved on"

if git -C "$repo2" merge other >/dev/null 2>&1; then
    t_fail "the merge fixture did not conflict; the scenario setup is wrong"
else
    merge_err="$(cd "$repo2" && "$TODO" resolve --file TODO.json 2>&1 >/dev/null)"
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
printf '%s\n' 'test-todo-resolve-rebase-message: PASS'
