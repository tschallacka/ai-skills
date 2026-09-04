#!/usr/bin/env bash
# MODE: DEV
# test-register-helpers.sh — the register writers (T41): add, update,
# close-with-evidence refusals, and the rebuild that repairs mechanical
# damage. Every writer runs against a throwaway copy of the real registers.
#
# It drives the COMPILED `bugs` and `todo` now. The four shell helpers it used
# to run -- planning/scripts/{bug,todo}-{add,update}.sh -- were deleted when
# those binaries replaced them, so every case exited 127 and the assertions
# below reported a missing field where the real cause was a missing file
# (B169). The wording of each refusal is quoted from the binaries' actual
# output rather than carried over from the scripts, because it differs: a task
# is "Queued" where it was "Added", and an out-of-vocabulary value is refused
# by the type at read time with the vocabulary listed.
#
# register-rebuild.sh is still a shell script and is still driven as one.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root_tests="$(cd "$tests_dir/../.." && pwd)"
scripts="$repo_root_tests/planning/scripts"
work="$(mktemp -d "${TMPDIR:-/tmp}/register-helpers.XXXXXX")"
trap 'rm -rf "$work"' EXIT

fail() { printf 'register-helpers: %s\n' "$1" >&2; FAILED=1; }
FAILED=0

# The binaries live under a per-triple directory and nothing puts them on PATH.
# A checkout that has not built them cannot run this, so it skips with the fix
# named rather than failing on absence -- the same shape the other tests use.
case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64)   triple=x86_64-unknown-linux-musl ;;
    Linux:aarch64|Linux:arm64)  triple=aarch64-unknown-linux-musl ;;
    Darwin:x86_64)              triple=x86_64-apple-darwin ;;
    Darwin:arm64)               triple=aarch64-apple-darwin ;;
    MINGW*|MSYS*|CYGWIN*|Windows*) triple=x86_64-pc-windows-msvc ;;
    *) triple='' ;;
esac
# The SKILL directory first, then the shared root. That order is not cosmetic:
# CI places them at <skill>/bin/<triple>/ because skill_files() lists that row,
# and a test that looked only in the root bin/ would SKIP on every CI leg --
# turning a red test into a silent pass, which is worse than the failure it
# replaced. setup-dev-env.sh and a plain `cargo build` land in the root, hence
# the fallback.
resolve_register_bin() { # <skill> <name>
    local candidate
    for candidate in "$repo_root_tests/$1/bin/$triple/$2" "$repo_root_tests/bin/$triple/$2"; do
        if [ -x "$candidate" ]; then printf '%s\n' "$candidate"; return 0; fi
    done
    return 1
}
bugs_bin=''
todo_bin=''
if [ -n "$triple" ]; then
    bugs_bin="$(resolve_register_bin bug-report bugs || true)"
    todo_bin="$(resolve_register_bin todo todo || true)"
fi
if [ -z "$bugs_bin" ] || [ -z "$todo_bin" ]; then
    echo "SKIP: the register binaries are not built for this host; run ./setup-dev-env.sh"
    exit 0
fi

todo="$work/TODO.json"
bugs="$work/BUGS.json"
cp "$repo_root_tests/TODO.json" "$todo"
cp "$repo_root_tests/BUGS.json" "$bugs"

run_todo() { "$todo_bin" --file "$todo" add "$@" 2>&1 || echo "rc=$?"; }
run_todoup() { "$todo_bin" --file "$todo" update "$@" 2>&1 || echo "rc=$?"; }
run_bugadd() { "$bugs_bin" --file "$bugs" add "$@" 2>&1 || echo "rc=$?"; }
run_bugup() { "$bugs_bin" --file "$bugs" update "$@" 2>&1 || echo "rc=$?"; }
# The soundness check the writers share, asked of a file from outside them.
reg_findings_out() {
    "$BASH" -c 'source "$1"; reg_findings "$2" "$3"' _ "$scripts/register-lib.sh" "$1" "$2"
}

# ---- add a task: it lands with stamps and sorts into place ------------------
out="$(run_todo --id T9999 --title 'Helper probe' --detail 'probe detail' --priority high)"
case "$out" in *'Queued T9999'*) : ;; *) fail "todo add did not report the add: $out" ;; esac
rjq -e --arg id T9999 '.tasks[] | select(.id == $id and .status == "open"
    and (.created_at | length > 0) and (.updated_at | length > 0))' "$todo" >/dev/null \
    || fail "the added task lacks its fields or stamps"

# ---- duplicate ids are refused ----------------------------------------------
out="$(run_todo --id T9999 --title 'Second of the same' --detail d)"
case "$out" in *'duplicate ids'*|*rc=65*) : ;; *) fail "a duplicate task id was accepted: $out" ;; esac

# ---- set-status done without evidence is refused; with note it lands --------
out="$(run_todoup T9999 --status "done" --note 'evidence: verified by suite')"
case "$out" in
        *'without evidence'*|*rc=65*) fail "done with a note was refused: $out" ;;
    esac
rjq -e --arg id T9999 '.tasks[] | select(.id == $id) | .status == "done"' "$todo" >/dev/null \
    || fail "set-status did not take effect"

# ---- unknown statuses are refused by the shared checks ----------------------
out="$(run_todoup T9999 --status finished)"
case "$out" in *'is not one of'*|*rc=65*) : ;; *) fail "an invented status was accepted: $out" ;; esac

# ---- a refused bug-update leaves the register byte-identical (B109) ---------
# The refusal must be atomic: the value that fails the shared checks must not
# already be in the file. cmp is the whole point of the case — a message alone
# proves nothing, since the pre-fix script printed one and wrote anyway.
probe_id="$(rjq -r '.bugs[0].id' "$bugs")"
cp "$bugs" "$work/bugs-before-refusal.json"
out="$(run_bugup "$probe_id" --priority bogus)"
case "$out" in *'is not one of'*|*rc=65*) : ;; *) fail "an invented priority was accepted: $out" ;; esac
cmp -s "$work/bugs-before-refusal.json" "$bugs" \
    || fail "a refused bug-update wrote to the register anyway (B109)"
[ -z "$(reg_findings_out bug "$bugs")" ] \
    || fail "a refused bug-update left the register unsound (B109)"

# ---- bug-add requires the four truth fields ---------------------------------
out="$(run_bugadd --title 'No reproduction attached')"
case "$out" in *rc=64*|*'required'*) : ;; *) fail "a defect without reproduction was accepted at the door: $out" ;; esac

# ---- a full defect files, sorts, and reports its id -------------------------
before="$(rjq '.bugs | length' "$bugs")"
out="$(run_bugadd --title 'Helper probe defect' \
    --reproduce 'bash planning/tests/test-register-helpers.sh' \
    --observed 'this line exists only so the case has something to observe' \
    --expected 'probe entries never appear in a real run' \
    --severity minor --surfaces 'planning/scripts/a.sh,planning/scripts/b.sh')"
case "$out" in *'Filed B'*) : ;; *) fail "bug-add did not report an id: $out" ;; esac
after="$(rjq '[.bugs[]] | length' "$bugs")"
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
rjq -e --arg id "$bid" '.bugs[] | select(.id == $id)
    | .status == "fixed" and (.fix | length > 0) and (.verification | length > 0)' "$bugs" >/dev/null \
    || fail "the closed entry lost its fix or verification text"

# ---- a damaged register is refused, and the damage is named -----------------
# The shell helper used to name register-rebuild.sh in the refusal; the binary
# names the DAMAGE instead ("no reproduction — a report without one is a
# rumour") and states that the file is unchanged. That is the load-bearing
# half, so it is what is asserted. The lost repair hint is a real if small
# regression in helpfulness, recorded rather than silently accepted: a reader
# who has never met register-rebuild.sh is no longer told it exists.
rjq '.bugs[-1].reproduce = ""' "$bugs" > "$work/dmg.json" && mv "$work/dmg.json" "$bugs"
cp "$bugs" "$work/bugs-before-damage-refusal.json"
out="$(run_bugup "$bid" --priority high)"
case "$out" in
    *'no reproduction'*) : ;;
    *) fail "a damaged register was accepted without naming the damage: $out" ;;
esac
case "$out" in
    *'is unchanged'*) : ;;
    *) fail "a refusal on a damaged register did not say the file was left alone: $out" ;;
esac
cmp -s "$work/bugs-before-damage-refusal.json" "$bugs" \
    || fail "a refused update on a damaged register wrote to it anyway"

# ---- the rebuild repairs what stamps can and refuses what it cannot ---------
out="$("$BASH" "$scripts/register-rebuild.sh" bugs "$bugs" 2>&1 || true)"
case "$out" in
    *'no reproduction'*) : ;;
    *) fail "the rebuild stayed silent about damage it cannot invent: $out" ;;
esac

# A stamp-repairable register (missing timestamps only) rebuilds clean. Built
# from the PRISTINE register: the rebuild refuses to invent reproductions, so
# a file carrying the earlier semantic damage must stay refused.
rjq '{skill, skill_version, comment, bugs: [.bugs[] | .created_at = "" | .updated_at = ""]}' \
    "$repo_root_tests/BUGS.json" > "$work/stamps.json"
out="$("$BASH" "$scripts/register-rebuild.sh" bugs "$work/stamps.json" 2>&1 || true)"
case "$out" in
    *'rebuilt'*'sound') : ;;
    *) fail "the rebuild refused a stamp-only repair: $out" ;;
esac
stamped="$(rjq '[.bugs[] | select(.created_at != "" and .updated_at != "")] | length' "$work/stamps.json")"
total="$(rjq '.bugs | length' "$work/stamps.json")"
[ "$stamped" -eq "$total" ] || fail "the rebuild left empty timestamps behind"

# ---- every accepted flag is exercised at least once (flag coverage) -------
# The damage-repair section above leaves the fixture deliberately scarred, so
# the flag probes start from fresh copies of the real registers.
cp "$repo_root_tests/TODO.json" "$todo"
cp "$repo_root_tests/BUGS.json" "$bugs"
# Not silenced. This add is only scaffolding for the --parent probe below, but
# `>/dev/null 2>&1` on it is how a real failure hid: `todo add` requires
# --detail, this call did not pass one, and the refusal surfaced three cases
# later as "parent T9999 does not exist" -- a downstream symptom blaming the
# wrong thing. A setup step that can fail is asserted like any other.
out="$(run_todo --id T9999 --title 'Flag probe parent' --detail 'parent for the flag probe')"
case "$out" in *'Queued T9999'*) : ;; *) fail "the flag probe's parent task was refused: $out" ;; esac
out="$(run_todo --id T8888 --title 'Flag probe' --parent T9999 --priority low \
    --blocked-on T9999 --detail 'flag detail' --refs planning/scripts/register-rebuild.sh)"
case "$out" in *'Queued T8888'*) : ;; *) fail "flag-rich todo-add refused: $out" ;; esac
rjq -e --arg id T8888 '.tasks[] | select(.id == $id
    and .parent == "T9999" and .blocked_on == "T9999"
    and .detail == "flag detail"
    and (.refs | index("planning/scripts/register-rebuild.sh") != null))' "$todo" >/dev/null \
    || fail "todo-add flags (--parent/--blocked-on/--detail/--ref) did not all land"
out="$(run_todoup T8888 --detail 'detail two' --blocked-on —)"
case "$out" in *Updated*) : ;; *) fail "todo-update --detail/--blocked-on refused: $out" ;; esac

out="$(run_bugadd --title 'Flag probe defect' --reproduce 'bash x' --observed o \
    --expected e --severity minor --mechanism 'off-by-one loop' \
    --parent B2 --surfaces 'planning/scripts/a.sh')"
bid="$(printf '%s' "$out" | grep -oE 'B[0-9]+' | head -1)"
case "$out" in *'Filed '"$bid"*) : ;; *) fail "flag-rich bug-add refused: $out" ;; esac
rjq -e --arg id "$bid" '.bugs[] | select(.id == $id
    and .mechanism == "off-by-one loop" and .parent == "B2")' "$bugs" >/dev/null \
    || fail "bug-add flags (--mechanism/--parent) did not all land"
out="$(run_bugup "$bid" --reason 'probe reason' --mechanism 'second mechanism' --append-note 'appended')"
case "$out" in *Updated*) : ;; *) fail "bug-update --reason/--mechanism/--append-note refused: $out" ;; esac

[ "$FAILED" -eq 0 ] || exit 1
printf '%s\n' 'test-register-helpers: PASS'
