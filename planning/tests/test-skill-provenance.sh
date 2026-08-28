#!/usr/bin/env bash
# MODE: DEV
# test-skill-provenance — a plan says which build of the skill created it, and
# an installed copy behind a reachable canonical checkout says so.
#
# Usage: test-skill-provenance.sh
#
# The failure was silent (T52): an installed copy 290 lines adrift of canonical
# behaved like a working skill, and the drift surfaced only when a reader
# concluded the reader itself lacked a feature and hand-patched around it.
# Nothing said which build was speaking. Silence is asserted here as strictly as
# the warning — a staleness warning that fires without evidence is worse than
# none, because the response to it is to reinstall.
#
# This file is shipped, so it holds to the shipped-runtime dependency rule in
# CODE-STYLE.md §1: bash, POSIX coreutils, awk, sed, grep, jq only. No python3.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

note_fail() { printf 'skill-provenance: %s\n' "$1" >&2; t_record "$1"; }
assert_has() { case "$2" in *"$1"*) ;; *) note_fail "$3: expected '$1' in: $2" ;; esac; }

work="$(mktemp -d "${TMPDIR:-/tmp}/skill-provenance.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# An installed copy is the skill without a git directory, plus the .version the
# installer writes. Built from the real tree so the test exercises the shipped
# scripts rather than a stand-in.
install="$work/installed"
mkdir -p "$install"
cp -r "$root/planning/scripts" "$install/"
cp "$root"/planning/*.json "$install/" 2>/dev/null || true
old_commit="$(git -C "$root" rev-parse --short HEAD~5 2>/dev/null || printf 'deadbee')"
printf 'format=ai-skills-version-1\npackage_version=9.9.9\nsource_version=branch:x commit:%s\n' \
    "$old_commit" > "$install/.version"

mkdir -p "$work/plans"
create() { PLANS_ROOT="$work/plans" "$1/scripts/create-plan.sh" "$2" "A probe plan" 2>&1; }

# 1. A checkout reports its commit, which is exact.
out="$(create "$root/planning" fromcheckout)"
assert_has 'planning skill: checkout at' "$out" 'a checkout names itself'

# 2. An installed copy reports the build the installer recorded, worded so it is
#    not mistaken for a statement about what canonical holds now.
out="$(create "$install" frominstall)"
assert_has 'installed build 9.9.9' "$out" 'an installed copy names its build'
assert_has "from commit $old_commit" "$out" 'an installed copy names its source commit'

# 3. With no canonical checkout named, no drift is claimed. There is nothing to
#    compare against, and guessing would send someone to reinstall for nothing.
case "$out" in
    *'behind'*) note_fail "drift was claimed with no canonical checkout to compare against" ;;
esac

# 4. With one reachable, the distance is named. This is the line whose absence
#    let a 290-line-adrift copy look healthy.
out="$(PLANS_ROOT="$work/plans" AI_SKILLS_REPO="$root" "$install/scripts/create-plan.sh" withrepo "A probe plan" 2>&1)"
assert_has 'commit(s) behind' "$out" 'drift against a reachable checkout is named'

# 5. A checkout is never reported as drifted against itself.
out="$(PLANS_ROOT="$work/plans" AI_SKILLS_REPO="$root" "$root/planning/scripts/create-plan.sh" selfcheck "A probe plan" 2>&1)"
case "$out" in
    *'behind'*) note_fail "a checkout was reported as behind itself" ;;
esac

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'skill-provenance: PASS\n'
