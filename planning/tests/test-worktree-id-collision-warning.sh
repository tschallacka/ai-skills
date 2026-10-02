#!/usr/bin/env bash
# MODE: DEV
# test-worktree-id-collision-warning.sh — B78: a linked git worktree has its
# own copy of the register, so an id minted there can collide with one minted
# concurrently in another worktree. reg_next_id cannot prevent that collision
# (it has no way to see the other worktree), so it warns on stderr instead,
# naming which worktree it read and the recovery command (bugs/todo resolve).
# The bare id on stdout is unaffected either way.
set -euo pipefail
export LC_ALL=C
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin
fail() { t_fail "$*"; }

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root_tests="$(cd "$tests_dir/../.." && pwd)"
reader="$repo_root_tests/planning/scripts/register-read.sh"

command -v git >/dev/null 2>&1 || {
    t_skip 'no git on PATH'
    exit 0
}

work="$(mktemp -d "${TMPDIR:-/tmp}/worktree-id-warning.XXXXXX")"
trap 'rm -rf "$work"' EXIT

repo="$work/repo"
mkdir -p "$repo"
git -C "$repo" init -q
git -C "$repo" config user.email test@example.com
git -C "$repo" config user.name test
cat > "$repo/BUGS.json" <<'JSON'
{"comment":"fixture","skill":"bug-report","skill_version":"2.0.0-alpha.1",
 "bugs":[{"id":"B5","title":"x","status":"reported","severity":"minor","priority":"low"}]}
JSON
git -C "$repo" add BUGS.json
git -C "$repo" commit -q -m fixture

# --- the main checkout: no warning, since there is nowhere else to collide with ---
out="$("$reader" bug next-id --file "$repo/BUGS.json" 2>"$work/main.err")"
[ "$out" = "6" ] || fail "main checkout: expected next id 6, got '$out'"
[ -s "$work/main.err" ] && fail "main checkout: unexpected stderr: $(cat "$work/main.err")"

# --- a linked worktree: the same file, from git's point of view, is a
#     different copy of the register than whatever the main checkout holds ---
git -C "$repo" worktree add -q -b wt-branch "$work/wt" >/dev/null
out="$("$reader" bug next-id --file "$work/wt/BUGS.json" 2>"$work/wt.err")"
[ "$out" = "6" ] || fail "linked worktree: expected next id 6 on stdout, got '$out'"
[ -s "$work/wt.err" ] || fail "linked worktree: expected a collision warning on stderr, got none"
grep -Fq 'linked git worktree' "$work/wt.err" || fail "warning does not name the worktree condition"
grep -Fq 'bugs/todo resolve' "$work/wt.err" || fail "warning does not name the recovery command"

t_end
