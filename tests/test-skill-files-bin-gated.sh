#!/usr/bin/env bash
# MODE: DEV
# test-skill-files-bin-gated.sh — a skill_files() bin/ artifact never crashes
# an install just because it has not been built.
#
# B317: chat, todo and bug-report each named their per-target `bin/...` binary
# in skill_files() with a bare `printf '%s\n' 'bin/...'`, unconditionally --
# whether or not that file actually existed. install_skill copies every path
# skill_files() names with a plain `cp`, and a fresh checkout with the binary
# not yet built gave that `cp` a source that did not exist. Under `set -e`
# that one failure killed the WHOLE install, not just the skill that named it
# -- every skill still queued behind it (in SKILL_NAMES order) silently never
# installed, with nothing in the log past a raw `cp: cannot stat` line.
#
# ai-text-editor and interactive-shell never had this defect: both route their
# bin/ rows through skill_artifact_files(), which prints a path only when
# `[ -f ... ]` finds it on disk, so a missing binary is quietly left out of the
# copy list rather than handed to `cp` as a promise. This test holds every
# skill to that same rule, so a new skill (or a new binary on an existing one)
# added the unsafe way is caught here instead of on a CI runner that has not
# built it yet.
#
# One exemption, named rather than path-matched: bin/*/rjq and bin/*/rjq.exe,
# which planning's arm still lists with a bare printf. That row is protected a
# different way -- install_skill's bundled_bin_row_missing() special-cases
# exactly those two filenames and skips the copy with a notice instead of
# calling cp on them at all (pre-existing behaviour, not part of B317). This
# test does not touch that; it only exempts what that mechanism already
# covers, so a new bin/ row copied from the rjq lines without also copying the
# protection is still caught.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
manifest="$repo_root/installer/src/50-manifest.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

[ -f "$manifest" ] || t_fail "no such file: $manifest"

# The body of skill_files() only: printf 'bin/...' lines elsewhere in the file
# (bundled_rjq_artifact in 20-runtime-tools.sh is a different function
# entirely, reached through a separate call path already exercised by
# test-installer-skill-selection.sh and friends) are out of scope here.
body="$(awk '
    /^skill_files\(\) \{/ { flag = 1 }
    flag { print }
    flag && /^}/ { exit }
' "$manifest")"

[ -n "$body" ] || t_fail "could not find skill_files() in $manifest -- has it moved or been renamed?"

# A bare `printf ... 'bin/...'` not routed through skill_artifact_files, and
# not one of the two rjq rows install_skill already guards a different way.
violations="$(printf '%s\n' "$body" \
    | grep -n "'bin/" \
    | grep -v 'skill_artifact_files' \
    | grep -vE "/rjq(\.exe)?'" \
    || true)"

t_assert_eq 'every skill_files() bin/ artifact is existence-gated (B317)' \
    "$violations" ''

t_end
