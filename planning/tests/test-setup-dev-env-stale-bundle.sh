#!/usr/bin/env bash
# MODE: DEV
# test-setup-dev-env-stale-bundle — B401: a generated plan-core-lib.sh that
# predates 050aa18c (B391) carries a pecbip_pick that never looks in
# <checkout>/bin/<triple>/ at all, so on a machine with skills installed
# globally it silently prefers an older SHARED binary over this checkout's
# own -- and setup-dev-env.sh's own "missing is built, present is left
# alone" policy for the OTHER four generated libraries never rebuilds a
# bundle that already exists, merely stale, so the stale bundle keeps
# choosing the implementation that keeps it stale, across every later pull.
#
# setup-dev-env.sh's own compiled-binary-preference block now checks
# build-plan-libs.sh --check before trusting plan-core-lib.sh, and repairs
# it first if stale -- this is the one call site where that is correct
# rather than drift-masking, since plan-core-lib.sh governs which
# implementation (compiled or bash) even answers the question, and none of
# the five plan-*-lib.sh files are committed for staleness to hide from a
# reviewer in the first place.
#
# This test mutates the REAL repo tree's own (gitignored, fully regenerable)
# planning/scripts/plan-core-lib.sh to reproduce the exact stale bundle, then
# restores it unconditionally on exit -- the most faithful reproduction of
# the bug, since the bootstrap block under test is inline in setup-dev-env.sh
# itself and reads that exact path, not a path a sandboxed copy could stand in
# for.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
lib="$scripts_dir/plan-core-lib.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

if [ ! -f "$lib" ]; then
    t_skip 'planning/scripts/plan-core-lib.sh does not exist yet (genuinely fresh checkout); nothing to stale'
fi

# shellcheck source=planning/scripts/plan-core-lib.sh
source "$lib"
# shellcheck source=planning/scripts/plan-crypt-lib.sh
source "$scripts_dir/plan-crypt-lib.sh"
triple="$(plan_crypt_target_triple)"
compiled_setup_dev_env="$repo_root/bin/$triple/setup-dev-env"
if [ ! -x "$compiled_setup_dev_env" ]; then
    t_skip 'no compiled bin/<triple>/setup-dev-env in this tree yet; the live run below would build the whole workspace'
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/setup-dev-env-stale-bundle.XXXXXX")"
backup="$work/plan-core-lib.sh.orig"
cp "$lib" "$backup"
restore() {
    cp "$backup" "$lib"
    rm -rf "$work"
}
trap restore EXIT

# Splice in the exact pre-050aa18c pecbip_pick: no devbin/devroot/triple
# locals at all, and a two-entry search order ("$1" "$side") that never
# consults the checkout's own bin/<triple> ahead of plan_bin_dir's shared
# bin. Reproduces "a bundle generated before 050aa18c" without checking out
# the historical commit, which would also need the four sibling templates
# that commit touched to stay internally consistent.
python3 -I - "$lib" <<'PY'
import re
import sys

path = sys.argv[1]
text = open(path).read()
old = '''pecbip_pick() {
    local dir candidate side devbin devroot triple
    side="$(cd "$3" && pwd)"
    devbin=""
    if [ -z "${AI_SKILLS_BIN_ROOT:-}" ]; then
        devroot="$(pecbip_find_skill_root "$side")" && triple="$(plan_crypt_target_triple 2>/dev/null)" \\
            && devbin="$devroot/bin/$triple"
    fi
    for dir in "$devbin" "$side" "$1"; do'''
new = '''pecbip_pick() {
    local dir candidate side
    side="$(cd "$3" && pwd)"
    for dir in "$1" "$side"; do'''
assert old in text, "pecbip_pick's current body did not match what this test expects to replace -- it may have changed shape since this test was written"
open(path, "w").write(text.replace(old, new, 1))
PY

t_assert_eq 'the deliberately-staled file has no devbin reference' "$(grep -c devbin "$lib" || true)" 0
stale_rc=0
planning/scripts/build-plan-libs.sh --check >/dev/null 2>&1 || stale_rc=$?
t_assert_eq 'build-plan-libs.sh --check correctly reports the staled file as stale (reproduces B401)' "$stale_rc" 1

# The actual regression check: setup-dev-env.sh's own bootstrap block must
# detect and repair this BEFORE dispatching, with no args beyond --check so
# this stays as a report-only, fast run.
check_out="$work/check.out"
check_rc=0
( cd "$repo_root" && ./setup-dev-env.sh --check ) >"$check_out" 2>&1 || check_rc=$?
t_assert_eq 'setup-dev-env.sh --check still exits cleanly despite the stale bundle' "$check_rc" 0
t_assert_eq 'plan-core-lib.sh is fresh again (devbin restored) after one setup-dev-env.sh run' \
    "$(grep -c devbin "$lib" || true)" 4
fresh_rc=0
planning/scripts/build-plan-libs.sh --check >/dev/null 2>&1 || fresh_rc=$?
t_assert_eq 'build-plan-libs.sh --check now passes: the repair matches a fresh build of the current templates, not merely the pre-test backup' "$fresh_rc" 0

t_end
