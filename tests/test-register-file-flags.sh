#!/usr/bin/env bash
# MODE: DEV
# test-register-file-flags.sh - the todo and bugs binaries' `--*-file` flags,
# end to end through the real binary. `Args::resolve_file_flag` (src/cli.rs,
# shared byte-identical by both crates) is unit-tested for the on-disk-path
# case; the `-` stdin path is not, because a Rust unit test has no shell
# stdin to read from. This drives the built binaries themselves, piping into
# stdin, so that path is actually exercised. A missing cargo and no prebuilt
# bin/ binaries is a loud SKIP, not a failure (mirrors chat/tests/test-chat.sh).

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$root/planning/tests/lib-test.sh"
t_begin

export LC_ALL=C
work="$(mktemp -d "${TMPDIR:-/tmp}/register-file-flags.XXXXXX")"
trap 'rm -rf "$work"' EXIT

package_version="$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    "$root/package.json" | head -1)"

TODO="$root/target/release/todo"
BUGS="$root/target/release/bugs"

if ! command -v cargo >/dev/null 2>&1; then
    prebuilt_todo="$(ls "$root"/todo/bin/*/todo 2>/dev/null | head -1 || true)"
    prebuilt_bugs="$(ls "$root"/bug-report/bin/*/bugs 2>/dev/null | head -1 || true)"
    if [ -n "$prebuilt_todo" ] && [ -n "$prebuilt_bugs" ]; then
        TODO="$prebuilt_todo"
        BUGS="$prebuilt_bugs"
    else
        t_skip 'test-register-file-flags: no cargo and no prebuilt bin/ binaries - rust assertions did not run'
    fi
else
    ( cd "$root/src/todo" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build todo failed"
    ( cd "$root/src/bug-report" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build bug-report failed"
fi

# ── todo: --note-file from stdin, via `-` ───────────────────────────────────
todo_file="$work/TODO.json"
cat > "$todo_file" <<JSON
{
  "skill": "todo",
  "skill_version": "$package_version",
  "comment": "seeded for test-register-file-flags.sh",
  "tasks": []
}
JSON
note_id="$("$TODO" --file "$todo_file" add --title "Prose that needs a file" \
    --detail "exercise --detail-file and --note-file" --priority normal)"
note_id="${note_id#Queued }"
note_id="$(printf '%s' "$note_id" | tr -d '[:space:]')"

printf 'a `note` with $shell-hostile characters\nand a second line' \
    | "$TODO" --file "$todo_file" update "$note_id" --status partly --note-file - \
    >/dev/null

expected_note_line='  "note": "a `note` with $shell-hostile characters\nand a second line",'
t_assert_eq 'todo: --note-file - reads stdin verbatim into note' \
    "$("$TODO" --file "$todo_file" show "$note_id" | grep '"note"')" \
    "$expected_note_line"

# ── todo: --note and --note-file together are refused ───────────────────────
# bash 3.2's ERR trap fires through a plain `cmd || rc=$?` guard even though
# newer bash exempts it; a subshell that resets the trap and always itself
# exits 0 (the pattern tests/test-skill-files-manifest.sh's probe_rc uses) is
# what stays quiet on every bash this repo runs under.
conflict_result="$(printf 'from stdin' | (
    trap - ERR
    set +e
    out="$("$TODO" --file "$todo_file" update "$note_id" --note "from argv" --note-file - 2>&1)"
    printf '%s\n%s' "$?" "$out"
))"
conflict_status="${conflict_result%%$'\n'*}"
conflict_out="${conflict_result#*$'\n'}"
[ "$conflict_status" -ne 0 ] || t_fail "todo: --note and --note-file together should be refused, exit was 0"
case "$conflict_out" in
    *note-file*) : ;;
    *) t_fail "todo: refusal did not name --note-file: $conflict_out" ;;
esac

# ── bugs: --fix-file and --verification-file from stdin, via `-` ───────────
bugs_file="$work/BUGS.json"
cat > "$bugs_file" <<JSON
{
  "skill": "bug-report",
  "skill_version": "$package_version",
  "comment": "seeded for test-register-file-flags.sh",
  "bugs": []
}
JSON
bug_id="$("$BUGS" --file "$bugs_file" add --title "Prose that needs a file" \
    --reproduce "r" --observed "o" --expected "e" --severity minor --priority normal \
    --status confirmed --mechanism "m")"
bug_id="${bug_id#Filed }"
bug_id="$(printf '%s' "$bug_id" | tr -d '[:space:]')"

printf 'a1b2c3d — fixed with a backtick ` in the message' \
    | "$BUGS" --file "$bugs_file" update "$bug_id" --status fixed \
        --fix-file - --verification "reproduced then re-run, now clean" \
    >/dev/null

expected_fix_line='  "fix": "a1b2c3d — fixed with a backtick ` in the message",'
t_assert_eq 'bugs: --fix-file - reads stdin verbatim into fix' \
    "$("$BUGS" --file "$bugs_file" show "$bug_id" | grep '"fix"')" \
    "$expected_fix_line"

t_end
