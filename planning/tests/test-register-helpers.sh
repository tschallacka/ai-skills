#!/usr/bin/env bash
# MODE: DEV
# test-register-helpers.sh — the register write helpers (T41): add, update,
# close-with-evidence refusals, and the rebuild that repairs mechanical
# damage. Every helper runs against a throwaway copy of the real registers.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root_tests="$(cd "$tests_dir/../.." && pwd)"
scripts="$repo_root_tests/planning/scripts"
work="$(mktemp -d "${TMPDIR:-/tmp}/register-helpers.XXXXXX")"
trap 'rm -rf "$work"' EXIT

fail() { printf 'register-helpers: %s\n' "$1" >&2; FAILED=1; }
FAILED=0

todo="$work/TODO.json"
bugs="$work/BUGS.json"
cp "$repo_root_tests/TODO.json" "$todo"
cp "$repo_root_tests/BUGS.json" "$bugs"

run_todo() { TODO_JSON="$todo" "$BASH" "$scripts/todo-add.sh" "$@" 2>&1 || echo "rc=$?"; }
run_todoup() { TODO_JSON="$todo" "$BASH" "$scripts/todo-update.sh" "$@" 2>&1 || echo "rc=$?"; }
run_bugadd() { BUGS_JSON="$bugs" "$BASH" "$scripts/bug-add.sh" "$@" 2>&1 || echo "rc=$?"; }
run_bugup() { BUGS_JSON="$bugs" "$BASH" "$scripts/bug-update.sh" "$@" 2>&1 || echo "rc=$?"; }

# ---- add a task: it lands with stamps and sorts into place ------------------
out="$(run_todo_add() { :; }; run_todo --id T9999 --title 'Helper probe' --priority high)"
case "$out" in *'Added T9999'*) : ;; *) fail "todo-add did not report the add: $out" ;; esac
jq -e --arg id T9999 '.tasks[] | select(.id == $id and .status == "open"
    and (.created_at | length > 0) and (.updated_at | length > 0))' "$todo" >/dev/null \
    || fail "the added task lacks its fields or stamps"

# ---- duplicate ids are refused ----------------------------------------------
out="$(run_todo --id T9999 --title 'Second of the same')"
case "$out" in *'duplicate ids'*|*rc=65*) : ;; *) fail "a duplicate task id was accepted: $out" ;; esac

# ---- set-status done without evidence is refused; with note it lands --------
out="$(run_todoup T9999 --status done --note 'evidence: verified by suite')"
case "$out" in *'without evidence'*|*rc=65*) fail "done with a note was refused: $out" ;; esac
jq -e --arg id T9999 '.tasks[] | select(.id == $id) | .status == "done"' "$todo" >/dev/null \
    || fail "set-status did not take effect"

# ---- unknown statuses are refused by the shared checks ----------------------
out="$(run_todoup T9999 --status finished)"
case "$out" in *'unknown status'*|*rc=65*) : ;; *) fail "an invented status was accepted: $out" ;; esac

# ---- bug-add requires the four truth fields ---------------------------------
out="$(run_bugadd --title 'No reproduction attached')"
case "$out" in *rc=64*|*'required'*) : ;; *) fail "a defect without reproduction was accepted at the door: $out" ;; esac

# ---- a full defect files, sorts, and reports its id -------------------------
before="$(jq '.bugs | length' "$bugs")"
out="$(run_bugadd --title 'Helper probe defect' \
    --reproduce 'bash planning/tests/test-register-helpers.sh' \
    --observed 'this line exists only so the case has something to observe' \
    --expected 'probe entries never appear in a real run' \
    --severity minor --surfaces 'planning/scripts/a.sh,planning/scripts/b.sh')"
case "$out" in *'Filed B'*) : ;; *) fail "bug-add did not report an id: $out" ;; esac
echo "PRE-COUNT: $(jq ".bugs|length" "$bugs")" >&2
after="$(jq '[.bugs[]] | length' "$bugs")"
[ "$after" -eq $((before + 1)) ] || fail "bug-add did not append exactly one entry"

# ---- closing as fixed without verification is refused -----------------------
bid="$(printf '%s' "$out" | grep -oE 'B[0-9]+' | head -1)"
out="$(run_bugup "$bid" --status fixed --fix 'pretend commit')"
case "$out" in *'requires --verification'*) : ;; *) fail "fixed without verification was accepted: $out" ;; esac

# ---- closing properly sets fix and verification -----------------------------
out="$(run_bugup "$bid" --status fixed \
    --fix 'probe commit — remove the seeded row' \
    --verification 'mutation: the seeded row reappears when this entry lies')"
case "$out" in *'Updated'*) : ;; *) fail "a well-evidenced close was refused: $out" ;; esac
jq -e --arg id "$bid" '.bugs[] | select(.id == $id)
    | .status == "fixed" and (.fix | length > 0) and (.verification | length > 0)' "$bugs" >/dev/null \
    || fail "the closed entry lost its fix or verification text"

# ---- a damaged register is refused, naming the rebuild ----------------------
jq '.bugs[-1].reproduce = ""' "$bugs" > "$work/dmg.json" && mv "$work/dmg.json" "$bugs"
cp "$bugs" /tmp/opencode/dbg-bugs.json
source "$scripts/register-lib.sh"
reg_findings bug /tmp/opencode/dbg-bugs.json > /tmp/opencode/dbg-find.txt 2>&1
out="$(run_bugup "$bid" --priority high)"
case "$out" in
    *'register-rebuild.sh'*) : ;;
    *) fail "a damaged register was accepted without naming the rebuild: $out" ;;
esac

# ---- the rebuild repairs what stamps can and refuses what it cannot ---------
out="$("$BASH" "$scripts/register-rebuild.sh" bugs "$bugs" 2>&1 || true)"
case "$out" in
    *'no reproduction'*) : ;;
    *) fail "the rebuild stayed silent about damage it cannot invent: $out" ;;
esac

# A stamp-repairable register (missing timestamps only) rebuilds clean. Built
# from the PRISTINE register: the rebuild refuses to invent reproductions, so
# a file carrying the earlier semantic damage must stay refused.
jq '{skill, skill_version, comment, bugs: [.bugs[] | .created_at = "" | .updated_at = ""]}' \
    "$repo_root_tests/BUGS.json" > "$work/stamps.json"
out="$("$BASH" "$scripts/register-rebuild.sh" bugs "$work/stamps.json" 2>&1 || true)"
case "$out" in
    *'rebuilt'*'sound') : ;;
    *) fail "the rebuild refused a stamp-only repair: $out" ;;
esac
stamped="$(jq '[.bugs[] | select(.created_at != "" and .updated_at != "")] | length' "$work/stamps.json")"
total="$(jq '.bugs | length' "$work/stamps.json")"
[ "$stamped" -eq "$total" ] || fail "the rebuild left empty timestamps behind"

# ---- every accepted flag is exercised at least once (flag coverage) -------
# The damage-repair section above leaves the fixture deliberately scarred, so
# the flag probes start from fresh copies of the real registers.
cp "$repo_root_tests/TODO.json" "$todo"
cp "$repo_root_tests/BUGS.json" "$bugs"
run_todo --id T9999 --title 'Flag probe parent' >/dev/null 2>&1
out="$(run_todo --id T8888 --title 'Flag probe' --parent T9999 --priority low \
    --blocked-on T9999 --detail 'flag detail' --ref planning/scripts/todo-add.sh)"
case "$out" in *'Added T8888'*) : ;; *) fail "flag-rich todo-add refused: $out" ;; esac
jq -e --arg id T8888 '.tasks[] | select(.id == $id
    and .parent == "T9999" and .blocked_on == "T9999"
    and .detail == "flag detail"
    and (.refs | index("planning/scripts/todo-add.sh") != null))' "$todo" >/dev/null \
    || fail "todo-add flags (--parent/--blocked-on/--detail/--ref) did not all land"
out="$(run_todoup T8888 --detail 'detail two' --blocked-on —)"
case "$out" in *Updated*) : ;; *) fail "todo-update --detail/--blocked-on refused: $out" ;; esac

out="$(run_bugadd --title 'Flag probe defect' --reproduce 'bash x' --observed o \
    --expected e --severity minor --mechanism 'off-by-one loop' \
    --parent B2 --surfaces 'planning/scripts/a.sh')"
bid="$(printf '%s' "$out" | grep -oE 'B[0-9]+' | head -1)"
case "$out" in *'Filed '"$bid"*) : ;; *) fail "flag-rich bug-add refused: $out" ;; esac
jq -e --arg id "$bid" '.bugs[] | select(.id == $id
    and .mechanism == "off-by-one loop" and .parent == "B2")' "$bugs" >/dev/null \
    || fail "bug-add flags (--mechanism/--parent) did not all land"
out="$(run_bugup "$bid" --reason 'probe reason' --mechanism 'second mechanism' --append-note 'appended')"
case "$out" in *Updated*) : ;; *) fail "bug-update --reason/--mechanism/--append-note refused: $out" ;; esac

[ "$FAILED" -eq 0 ] || exit 1
printf '%s\n' 'test-register-helpers: PASS'
