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
if ! command -v rjq >/dev/null 2>&1; then
    printf 'UNCONFIGURED (rjq)\n'
    exit 0
fi

check_register() { # <kind> <file>
    local kind="$1" file="$2" name findings count
    name="${file##*/}"
    if [ ! -f "$file" ]; then
        t_fail "$name is missing from the repository root"
        return
    fi
    if ! rjq -e '.' "$file" >/dev/null 2>&1; then
        t_fail "$name is not valid JSON — if it is mid-merge, resolve it with planning/scripts/register-resolve.sh"
        return
    fi
    findings="$(reg_findings "$kind" "$file")"
    if [ -n "$findings" ]; then
        count="$(printf '%s\n' "$findings" | wc -l | tr -d ' ')"
        t_fail "$name breaks $count of its own rules; the writers would have refused these:"
        printf '%s\n' "$findings" | sed 's/^/      /' >&2
        printf '    fix each entry through bug-update.sh / todo-update.sh, or run\n' >&2
        printf '    planning/scripts/register-rebuild.sh %s "%s" for structural damage\n' "$kind" "$file" >&2
        return
    fi
    printf '  %s: sound (%s entries)\n' "$name" \
        "$(rjq -r --arg k "$([ "$kind" = bug ] && echo bugs || echo tasks)" '.[$k] | length' "$file")"
}

check_register bug "$repo_root/BUGS.json"
check_register todo "$repo_root/TODO.json"

# No id-shape check lives here, and that is a measured decision rather than an
# omission. The writers take `--id` from the caller and only *suggest* the next
# free number, so there is no allocation rule to enforce: TODO.json legitimately
# carries 28 suffixed sub-task ids (T1e, T41a, T70a and the rest). A shape check
# flagged every one of them the first time it ran.

t_end 'test-register-soundness'
