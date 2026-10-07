#!/usr/bin/env bash
# MODE: DEV
# test-register-soundness.sh — the committed registers obey their own rules.
#
# BUGS.json and TODO.json are written by bug-add.sh / bug-update.sh /
# todo-add.sh / todo-update.sh, and every one of those refuses an entry that
# reg_findings would reject: an unknown severity, an unknown status, a parent
# that does not exist, a duplicate id, a missing timestamp, a bug with no
# reproduction, a confirmed bug with no mechanism, a fixed bug with no
# verification.
#
# So a finding here means more than "the register is malformed". It means the
# entry did not come from the writers at all, because they would have refused
# it — the register was edited by hand or by something imitating their output.
# That is worth failing the suite over: until this existed, reg_findings ran
# only inside the writers, so a hand-edited register could be committed, pushed
# and merged with CI green. It was a real merge that surfaced it.
#
# Deliberately NOT checked: whether an entry's key set matches what the writers
# emit. That was measured and rejected as a signal — an unsound register found
# in the wild had entries whose keys matched the writers' output exactly, and
# every one of the 101 bugs and 109 tasks here matches it too. A check that
# cannot separate a defect from correct usage does not get to fail the build
# (CODE-CONTRACTS.md contract 5); the rules above can, so they do.
#
# Usage:
#   test-register-soundness.sh

set -uo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
# shellcheck source=planning/scripts/register-lib.sh
source "$repo_root/planning/scripts/register-lib.sh"

t_begin

# reg_findings exits 69 without rjq rather than half-reading a register, so the
# absence is reported as unconfigured instead of as a red register.
if ! command -v jq >/dev/null 2>&1; then
    printf 'UNCONFIGURED (jq)\n'
    exit 0
fi

check_register() { # <kind> <file>
    local kind="$1" file="$2" name findings count
    name="${file##*/}"
    if [ ! -f "$file" ]; then
        t_fail "$name is missing from the repository root"
        return
    fi
    if ! jq -e '.' "$file" >/dev/null 2>&1; then
        t_fail "$name is not valid JSON — if it is mid-merge, resolve it with 'bugs resolve' / 'todo resolve'"
        return
    fi
    findings="$(reg_findings "$kind" "$file")"
    if [ -n "$findings" ]; then
        count="$(printf '%s\n' "$findings" | wc -l | tr -d ' ')"
        t_fail "$name breaks $count of its own rules; the writers would have refused these:"
        printf '%s\n' "$findings" | sed 's/^/      /' >&2
        printf '    fix each entry through the shipped bugs/todo binaries, or run\n' >&2
        printf '    register-rebuild %s "%s" for structural damage\n' "$kind" "$file" >&2
        return
    fi
    printf '  %s: sound (%s entries)\n' "$name" \
        "$(jq -r --arg k "$([ "$kind" = bug ] && echo bugs || echo tasks)" '.[$k] | length' "$file")"
}

check_register bug "$repo_root/BUGS.json"
check_register todo "$repo_root/TODO.json"

# No id-shape check lives here, and that is a measured decision rather than an
# omission. The writers take `--id` from the caller and only *suggest* the next
# free number, so there is no allocation rule to enforce: TODO.json legitimately
# carries 28 suffixed sub-task ids (T1e, T41a, T70a and the rest). A shape check
# flagged every one of them the first time it ran.

# DECISIONS.json has no real committed register at the repository root yet (no
# question has been filed against this repo itself), so there is nothing for
# check_register's real-file shape to exercise -- unlike BUGS.json/TODO.json,
# requiring it to exist would fail this suite for every checkout until someone
# happens to raise a question. Crafted fixtures instead: the same reg_findings
# call bugs/todo's own writers refuse against, proven here to catch a known-bad
# decision register and to pass a clean one, so the three-way kind dispatch
# (register-lib.sh) actually reaches .questions rather than silently reading
# .tasks or finding nothing.
decisions_work="$(mktemp -d "${TMPDIR:-/tmp}/decisions-soundness.XXXXXX")"
trap 'rm -rf "$decisions_work"' EXIT

cat > "$decisions_work/bad.json" <<'JSON'
{
  "skill": "decisions",
  "skill_version": "test",
  "questions": [
    { "id": "Q1", "title": "one", "status": "open", "priority": "urgent", "branch": "main",
      "options": [{"letter": "a", "label": "x"}],
      "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" },
    { "id": "Q2", "title": "", "status": "bogus", "priority": "normal", "branch": "",
      "options": [],
      "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" },
    { "id": "Q1", "title": "duplicate of Q1", "status": "closed", "priority": "low", "branch": "main",
      "options": [{"letter": "a", "label": "x"}],
      "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" }
  ]
}
JSON
bad_findings="$(reg_findings decision "$decisions_work/bad.json")"
t_assert_contains 'a crafted bad DECISIONS.json reports the duplicate id' 'duplicate ids' "$bad_findings"
t_assert_contains 'it reports the unknown status' 'Q2: unknown status bogus' "$bad_findings"
t_assert_contains 'it reports the missing title' 'Q2: missing title' "$bad_findings"
t_assert_contains 'it reports the missing options' 'Q2: no options' "$bad_findings"
t_assert_contains 'it reports the missing branch' 'Q2: missing branch' "$bad_findings"

cat > "$decisions_work/clean.json" <<'JSON'
{
  "skill": "decisions",
  "skill_version": "test",
  "questions": [
    { "id": "Q1", "title": "one", "status": "open", "priority": "urgent", "branch": "main",
      "options": [{"letter": "a", "label": "x"}],
      "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" },
    { "id": "Q3", "title": "three", "status": "closed", "priority": "low", "branch": "main",
      "options": [{"letter": "a", "label": "x"}],
      "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" }
  ]
}
JSON
clean_findings="$(reg_findings decision "$decisions_work/clean.json")"
t_assert_eq 'a clean DECISIONS.json reports no findings' "$clean_findings" ''

t_end 'test-register-soundness'
