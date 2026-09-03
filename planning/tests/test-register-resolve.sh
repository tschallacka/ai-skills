#!/usr/bin/env bash
# MODE: DEV
# test-register-resolve.sh — register-resolve.sh resolves an id collision, and
# refuses everything it cannot resolve honestly.
#
# The fixtures are real merge conflicts in throwaway repositories rather than
# hand-written conflict markers: the tool reads the index stages, so a fixture
# that only looks conflicted would exercise none of the code that matters.
#
# Usage:
#   test-register-resolve.sh

set -uo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
resolver="$tests_dir/../scripts/register-resolve.sh"

t_begin

if ! command -v rjq >/dev/null 2>&1; then
    printf 'UNCONFIGURED (rjq)\n'
    exit 0
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/register-resolve.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# ── fixture builders ────────────────────────────────────────────────────────
# bug_entry <id> <title> [severity]
bug_entry() {
    printf '{ "id": "%s", "title": "%s", "status": "reported", "severity": "%s",' \
        "$1" "$2" "${3:-minor}"
    printf ' "priority": "normal", "parent": null, "reproduce": "r",'
    printf ' "observed": "o", "expected": "e",'
    printf ' "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" }'
}

# task_entry <id> <title> <parent-json> <blocked-json> <detail-json>
task_entry() {
    printf '{ "id": "%s", "title": "%s", "status": "open", "priority": "normal",' "$1" "$2"
    printf ' "parent": %s, "blocked_on": %s, "detail": %s,' "$3" "$4" "$5"
    printf ' "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" }'
}

# register <kind> <entries-json-list>
register() {
    local key=bugs skill=bug-report
    [ "$1" = todo ] && { key=tasks; skill=todo; }
    printf '{ "skill": "%s", "skill_version": "1.4.2", "comment": "fixture", "%s": [%s] }\n' \
        "$skill" "$key" "$2"
}

# conflict <name> <kind> <file> <base-entries> <ours-entries> <theirs-entries>
# Leaves a repository at <work>/<name> mid-merge with <file> conflicted.
conflict() {
    local name="$1" kind="$2" file="$3" base="$4" ours="$5" theirs="$6" repo
    repo="$work/$name"
    mkdir -p "$repo"
    (
        cd "$repo" || exit 70
        git init -q .
        git config user.email test@example.invalid
        git config user.name test
        git config commit.gpgsign false
        register "$kind" "$base" > "$file"
        git add "$file" && git commit -qm base
        git checkout -qb ours
        register "$kind" "$ours" > "$file"
        git commit -qam ours
        git checkout -q master 2>/dev/null || git checkout -q main
        git checkout -qb theirs
        register "$kind" "$theirs" > "$file"
        git commit -qam theirs
        git checkout -q ours
        # The conflict is the point, so the merge's non-zero status is expected
        # and must not read as the fixture having failed.
        git merge theirs >/dev/null 2>&1 || true
    ) >/dev/null 2>&1
    printf '%s\n' "$repo"
}

# run <repo> <args...> — stdout to $out_file, stderr to $err_file, sets $rc
out_file="$work/stdout"
err_file="$work/stderr"
rc=0
run() {
    local repo="$1"; shift
    rc=0
    ( cd "$repo" && CONFIRM='' "$BASH" "$resolver" "$@" ) >"$out_file" 2>"$err_file" || rc=$?
}

# run_confirmed <repo> <hash> <args...> — the same, with the confirmation in
# the environment, which is the only way to hand one over.
run_confirmed() {
    local repo="$1" token="$2"; shift 2
    rc=0
    ( cd "$repo" && CONFIRM="$token" "$BASH" "$resolver" "$@" ) >"$out_file" 2>"$err_file" || rc=$?
}

# ── 1. a plain collision ────────────────────────────────────────────────────
repo="$(conflict collision bug BUGS.json \
    "$(bug_entry B01 'base one')" \
    "$(bug_entry B01 'base one'), $(bug_entry B50 'ours: sockets leak')" \
    "$(bug_entry B01 'base one'), $(bug_entry B50 'theirs: parser loops')")"

run "$repo" --check BUGS.json
t_assert_eq '--check reports a conflicted register with exit 1' "$rc" '1'
t_assert_contains '--check names the file' 'conflicted BUGS.json' "$(cat "$out_file")"

run "$repo" BUGS.json
t_assert_eq 'no decisions is an error' "$rc" '65'
t_assert_contains 'the undecided id is named' 'B50' "$(cat "$err_file")"
t_assert_contains 'both sides titles are shown' 'ours: sockets leak' "$(cat "$err_file")"
t_assert_contains 'and the other side too' 'theirs: parser loops' "$(cat "$err_file")"
t_assert_contains 'a free id is suggested' 'B51' "$(cat "$err_file")"

run "$repo" BUGS.json theirs:B50:B51
t_assert_eq 'a decided collision previews cleanly' "$rc" '0'
t_assert_eq 'the preview carries all three entries' \
    "$(rjq -r '.bugs | length' "$out_file")" '3'
t_assert_eq 'ours keeps the contested id' \
    "$(rjq -r '.bugs[] | select(.id == "B50") | .title' "$out_file")" 'ours: sockets leak'
t_assert_eq 'theirs is carried under the new id' \
    "$(rjq -r '.bugs[] | select(.id == "B51") | .title' "$out_file")" 'theirs: parser loops'
t_assert_eq 'the base entry is not duplicated' \
    "$(rjq -r '[.bugs[] | select(.id == "B01")] | length' "$out_file")" '1'

hash="$(sed -n 's/.*CONFIRM=\([0-9a-f]*\).*/\1/p' "$err_file" | tail -1)"
if [ -z "$hash" ]; then
    t_fail 'no confirm hash was printed'
else
    # The preview must not have written anything.
    t_assert_eq 'a preview leaves the register conflicted' \
        "$(cd "$repo" && git ls-files -u -- BUGS.json | wc -l | tr -d ' ')" '3'

    run_confirmed "$repo" 0000000000000000 BUGS.json theirs:B50:B51
    t_assert_eq 'a wrong confirmation is refused' "$rc" '65'
    t_assert_eq 'and writes nothing' \
        "$(cd "$repo" && git ls-files -u -- BUGS.json | wc -l | tr -d ' ')" '3'

    run_confirmed "$repo" "$hash" BUGS.json theirs:B50:B51
    t_assert_eq 'the printed confirmation writes' "$rc" '0'
    t_assert_eq 'the written register holds three entries' \
        "$(rjq -r '.bugs | length' "$repo/BUGS.json")" '3'
    t_assert_eq 'and carries no conflict markers' \
        "$(awk '/^<<<<<<<|^=======$|^>>>>>>>/ {n++} END {print n+0}' "$repo/BUGS.json")" '0'
fi

# ── 2. renaming follows parent and blocked_on, and reports prose ────────────
repo="$(conflict refs todo TODO.json \
    "$(task_entry T01 'root' null null null)" \
    "$(task_entry T01 'root' null null null), $(task_entry T10 'ours: widget' null null null), $(task_entry T11 'ours: child' '"T10"' '"T10"' '"follows T10 closely"')" \
    "$(task_entry T01 'root' null null null), $(task_entry T10 'theirs: parser' null null null)")"

run "$repo" TODO.json ours:T10:T20
t_assert_eq 'a todo collision previews cleanly' "$rc" '0'
t_assert_eq 'the renamed entry takes the new id' \
    "$(rjq -r '.tasks[] | select(.title == "ours: widget") | .id' "$out_file")" 'T20'
t_assert_eq 'a child parent follows the rename' \
    "$(rjq -r '.tasks[] | select(.id == "T11") | .parent' "$out_file")" 'T20'
t_assert_eq 'an id-valued blocked_on follows the rename' \
    "$(rjq -r '.tasks[] | select(.id == "T11") | .blocked_on' "$out_file")" 'T20'
t_assert_eq 'the other side keeps the contested id' \
    "$(rjq -r '.tasks[] | select(.id == "T10") | .title' "$out_file")" 'theirs: parser'
t_assert_contains 'a prose mention is reported' 'still mentions T10 in its prose' \
    "$(cat "$err_file")"
t_assert_eq 'and prose is NOT rewritten' \
    "$(rjq -r '.tasks[] | select(.id == "T11") | .detail' "$out_file")" 'follows T10 closely'

# ── 3. a divergence is refused, not renamed ─────────────────────────────────
repo="$(conflict divergence bug BUGS.json \
    "$(bug_entry B01 'shared' minor)" \
    "$(bug_entry B01 'shared, a parser fault' major)" \
    "$(bug_entry B01 'shared, a locale fault' blocking)")"

run "$repo" BUGS.json
t_assert_eq 'a divergence refuses' "$rc" '65'
t_assert_contains 'and says which entry diverged' 'divergence: B01' "$(cat "$err_file")"
t_assert_contains 'and why a rename is the wrong answer' 'would not' "$(cat "$err_file")"

# ── 4. decision validation ─────────────────────────────────────────────────
repo="$(conflict validation bug BUGS.json \
    "$(bug_entry B01 'base one')" \
    "$(bug_entry B01 'base one'), $(bug_entry B50 'ours')" \
    "$(bug_entry B01 'base one'), $(bug_entry B50 'theirs')")"

run "$repo" BUGS.json nosuch:B50:B51
t_assert_eq 'an unknown side is a usage error' "$rc" '64'
t_assert_contains 'naming the sides that do exist' 'ours, theirs' "$(cat "$err_file")"

run "$repo" BUGS.json ours:B50:T51
t_assert_eq 'a new id of the wrong register is refused' "$rc" '64'

run "$repo" BUGS.json ours:B50:B01
t_assert_eq 'a new id already in use is refused' "$rc" '64'
t_assert_contains 'and a free one is suggested' 'next free' "$(cat "$err_file")"

# ── 5. a clean register is a no-op, not an error ────────────────────────────
clean_repo="$work/clean"
mkdir -p "$clean_repo"
(
    cd "$clean_repo" || exit 70
    git init -q .
    git config user.email test@example.invalid
    git config user.name test
    register bug "$(bug_entry B01 'only one')" > BUGS.json
    git add BUGS.json && git commit -qm base
) >/dev/null 2>&1

run "$clean_repo" --check BUGS.json
t_assert_eq '--check on a clean register exits 0' "$rc" '0'
t_assert_contains 'and says so' 'clean BUGS.json' "$(cat "$out_file")"

run "$clean_repo" BUGS.json
t_assert_eq 'resolving a clean register is a no-op, not a failure' "$rc" '0'

# ── 6. what changed since the base: one-sided edits are named, shared are not ─
# The base carries B01 and B02. Ours edits B01's severity and both sides edit
# B02 the same way, so B01 must be reported as a one-sided edit naming the key
# and B02 must be reported as shared rather than as something to suspect.
repo="$(conflict sincebase bug BUGS.json \
    "$(bug_entry B01 'shared defect' minor), $(bug_entry B02 'other defect' minor)" \
    "$(bug_entry B01 'shared defect' major), $(bug_entry B02 'other defect' cosmetic), $(bug_entry B50 'ours new')" \
    "$(bug_entry B01 'shared defect' minor), $(bug_entry B02 'other defect' cosmetic), $(bug_entry B50 'theirs new')")"

run "$repo" BUGS.json theirs:B50:B51
t_assert_eq 'the since-base report does not stop a resolve' "$rc" '0'
report="$(cat "$err_file")"
t_assert_contains 'a one-sided edit is reported as such' 'ONLY' "$report"
t_assert_contains 'naming the entry that was edited' 'B01: severity' "$report"
t_assert_contains 'an identical edit on both sides is explained, not suspected' \
    'modified identically on both sides' "$report"
# B02 changed on both sides the same way, so it must not appear in a ONLY list.
only_block="$(printf '%s\n' "$report" | awk '/ONLY/,0')"
case "$only_block" in
    *B02*) t_fail 'B02 changed identically on both sides but was reported as one-sided' ;;
esac
t_assert_contains 'added entries are counted per side' 'added' "$report"

# ── 7. an edit only one side made survives, whichever side made it ──────────
# The union used to take ours first for every id it had, so a base entry that
# only theirs had edited came through at its BASE value and the edit vanished
# with no message. Both directions are asserted, because ours-only passing is
# what made the bug invisible.
repo="$(conflict onesided bug BUGS.json \
    "$(bug_entry B01 'edited by theirs' minor), $(bug_entry B02 'edited by ours' minor)" \
    "$(bug_entry B01 'edited by theirs' minor), $(bug_entry B02 'edited by ours' blocking), $(bug_entry B50 'ours new')" \
    "$(bug_entry B01 'edited by theirs' blocking), $(bug_entry B02 'edited by ours' minor), $(bug_entry B50 'theirs new')")"

run "$repo" BUGS.json theirs:B50:B51
t_assert_eq 'a one-sided edit on each side is not a divergence' "$rc" '0'
t_assert_eq "an edit only theirs made survives" \
    "$(rjq -r '.bugs[] | select(.id == "B01") | .severity' "$out_file")" 'blocking'
t_assert_eq "an edit only ours made survives" \
    "$(rjq -r '.bugs[] | select(.id == "B02") | .severity' "$out_file")" 'blocking'

t_end 'test-register-resolve'
