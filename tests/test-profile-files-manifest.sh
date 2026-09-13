#!/usr/bin/env bash
# MODE: DEV
# test-profile-files-manifest.sh — profile_files() lists what .agents/profiles/
# actually has, in both directions (T102).
#
# Mirrors tests/test-skill-files-manifest.sh's own two-directional check, cut
# down to what a profile actually is: one file per profile, no dev/prod split,
# no per-platform artifact. A hand list only works if something notices when
# it drifts:
#
#   listed but absent    profile_files() promises a file the profile does not
#                        have -- a rename that was not carried through here
#   present but unlisted a file added to .agents/profiles/ that no install
#                        step ever delivers, so it works for the author and
#                        nowhere else
#
# Both are checked against `git ls-files .agents/profiles`, the packaged set:
# `.agents` is in package.json's own files array, so a tracked file ships.

set -euo pipefail
export LC_ALL=C

# See test-skill-files-manifest.sh's own comment for why these three are
# unset before this check derives its own root from BASH_SOURCE.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_OBJECT_DIRECTORY

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

# shellcheck source=installer/src/05-config.sh
source "$repo_root/installer/src/05-config.sh"
# shellcheck source=installer/src/50-manifest.sh
source "$repo_root/installer/src/50-manifest.sh"

work="$(mktemp -d "${TMPDIR:-/tmp}/profile-files.XXXXXX")"
trap 'rm -rf "$work"' EXIT

[ "${#PROFILE_NAMES[@]}" -gt 0 ] || t_fail 'PROFILE_NAMES lists no profile'

( cd "$repo_root/.agents/profiles" && git ls-files | sort ) > "$work/tracked"
tracked_count="$(grep -c . < "$work/tracked" || true)"
[ "$tracked_count" -gt 0 ] || t_fail '.agents/profiles tracks nothing'

total_listed=0
: > "$work/listed_all"
for profile in "${PROFILE_NAMES[@]}"; do
    profile_files "$profile" | sort > "$work/listed"
    listed_count="$(grep -c . < "$work/listed" || true)"
    [ "$listed_count" -gt 0 ] || t_fail "$profile: profile_files() listed nothing"
    total_listed=$((total_listed + listed_count))
    cat "$work/listed" >> "$work/listed_all"

    absent=''
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        [ -e "$repo_root/.agents/profiles/$path" ] || absent="$absent $path"
    done < "$work/listed"
    t_assert_eq "$profile: every file profile_files() promises exists on disk" \
        "${absent# }" ''
done
sort -o "$work/listed_all" "$work/listed_all"

unlisted="$(comm -13 "$work/listed_all" "$work/tracked" | tr '\n' ' ')"
t_assert_eq 'every tracked profile file is declared by profile_files()' \
    "${unlisted% }" ''

printf '%s: accounted for %s listed file(s) across %s profile(s)\n' \
    "${0##*/}" "$total_listed" "${#PROFILE_NAMES[@]}" >&2

t_end
