#!/usr/bin/env bash
# MODE: DEV
# test-registers-guard.sh — the register branch's gate refuses what it must.
#
# The gate lets a push reach master with NO review, so every assertion here is
# about something it must refuse rather than something it must allow. The one
# that matters most is the duplicate id: git merges two entries that took the
# same next free id textually, with no conflict, at different array positions,
# so nothing but an explicit check can see it. Eight arrived that way in one
# merge on 2026-09-04.
#
# Each case is fault-injected: the fixture is built wrong on purpose, and the
# test asserts the guard says no AND says why. A gate whose refusals are not
# exercised is the failure mode this repo keeps finding in its own gates.
#
# Usage:
#   test-registers-guard.sh

set -uo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

guard="$repo_root/.github/registers-guard.sh"
[ -x "$guard" ] || { printf '%s: no %s\n' "${0##*/}" "$guard" >&2; exit 1; }

work="$(mktemp -d "${TMPDIR:-/tmp}/registers-guard.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# A minimal pair of registers, sound, in their own directory: the guard reads
# BUGS.json and TODO.json relative to the cwd, so the fixtures are cwd.
fixture() { # <dir>
    mkdir -p "$1"
    cat > "$1/BUGS.json" <<'JSON'
{
  "skill": "bug-report",
  "skill_version": "test",
  "bugs": [
    { "id": "B1", "title": "one", "parent": null },
    { "id": "B2", "title": "two", "parent": "B1" }
  ]
}
JSON
    cat > "$1/TODO.json" <<'JSON'
{
  "skill": "todo",
  "skill_version": "test",
  "tasks": [
    { "id": "T1", "title": "one", "parent": null }
  ]
}
JSON
}

run_guard() { # <dir> <changed-paths...> -> echoes exit code, output in $work/out
    local dir="$1"; shift
    local list="$work/changed"
    printf '%s\n' "$@" > "$list"
    local rc=0
    if ( cd "$dir" && "$guard" --files-from "$list" >"$work/out" 2>&1 ); then rc=0; else rc=$?; fi
    printf '%s' "$rc"
}

# ---- 1. a sound register change is allowed ---------------------------------
# The positive control. Without it, a guard that refused everything would
# satisfy every assertion below.
fixture "$work/sound"
rc="$(run_guard "$work/sound" BUGS.json)"
t_assert_eq 'a register-only change with sound registers is allowed' "$rc" '0'
t_assert_contains 'it says it may merge' 'may merge' "$(cat "$work/out")"

# ---- 2. a stray path is refused --------------------------------------------
# The load-bearing refusal: this branch reaches master without review, so it
# must not be able to carry code.
fixture "$work/stray"
rc="$(run_guard "$work/stray" BUGS.json README.md)"
t_assert_eq 'a change touching anything but a register is refused' "$rc" '1'
out="$(cat "$work/out")"
t_assert_contains 'the refusal names the offending path' 'README.md' "$out"
t_assert_contains 'the refusal explains the bypass risk' 'protection bypass' "$out"

# ---- 3. a duplicate id is refused, and named -------------------------------
# The check git cannot perform. The duplicate is placed at the END of the array
# rather than beside its twin, because that is what a textual merge produces
# and a naive adjacent-pair comparison would miss it.
fixture "$work/dupe"
python3 - "$work/dupe/BUGS.json" <<'PY'
import json, sys
p = sys.argv[1]
d = json.load(open(p))
d["bugs"].append({"id": "B1", "title": "an unrelated entry that took the same id", "parent": None})
json.dump(d, open(p, "w"), indent=2)
PY
rc="$(run_guard "$work/dupe" BUGS.json)"
t_assert_eq 'a duplicate id is refused' "$rc" '1'
out="$(cat "$work/out")"
t_assert_contains 'the refusal names the duplicated id' 'B1' "$out"
t_assert_contains 'the refusal names the cause' 'same next free id' "$out"

# ---- 4. a parent that does not resolve is refused --------------------------
fixture "$work/dangling"
python3 - "$work/dangling/BUGS.json" <<'PY'
import json, sys
p = sys.argv[1]
d = json.load(open(p))
d["bugs"][1]["parent"] = "B999"
json.dump(d, open(p, "w"), indent=2)
PY
rc="$(run_guard "$work/dangling" BUGS.json)"
t_assert_eq 'a parent that does not exist is refused' "$rc" '1'
t_assert_contains 'the refusal names the dangling link' 'B2->B999' "$(cat "$work/out")"

# ---- 5. an unparseable register is refused ---------------------------------
# Truncation rather than gibberish: a half-written register is what an
# interrupted write leaves behind, and it must not reach master.
fixture "$work/broken"
printf '{ "bugs": [ { "id": "B1"' > "$work/broken/BUGS.json"
rc="$(run_guard "$work/broken" BUGS.json)"
t_assert_eq 'a register that does not parse is refused' "$rc" '1'
t_assert_contains 'the refusal says it did not parse' 'does not parse' "$(cat "$work/out")"

# ---- 6. an empty change set is refused -------------------------------------
# A push that changes nothing has no business fast-forwarding master, and it is
# far likelier to mean the base resolved wrongly than a deliberate empty commit.
fixture "$work/empty"
: > "$work/changed"
if ( cd "$work/empty" && "$guard" --files-from "$work/changed" >"$work/out" 2>&1 ); then rc=0; else rc=$?; fi
t_assert_eq 'an empty change set is refused' "$rc" '1'
t_assert_contains 'the refusal says the change set was empty' 'empty' "$(cat "$work/out")"

# ---- 7. a base ref that does not resolve is refused, not guessed at --------
fixture "$work/badbase"
if ( cd "$work/badbase" && "$guard" --base refs/heads/definitely-not-a-ref >"$work/out" 2>&1 ); then rc=0; else rc=$?; fi
t_assert_eq 'an unresolvable base is refused' "$rc" '1'
t_assert_contains 'the refusal says the base did not resolve' 'does not resolve' "$(cat "$work/out")"

t_end 'test-registers-guard'
