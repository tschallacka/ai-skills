#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: DEV
# test-skill-files-manifest.sh — skill_files() lists what the skill actually has.
#
# skill_files() is a hand-written list, on purpose: the planning arm is a second
# copy of PACKAGE-MANIFEST.txt and the duplication IS the cross-check. A hand
# list only works if something notices when it drifts, in either direction:
#
#   listed but absent    an installed skill missing a file it was promised --
#                        a rename or a version bump nobody carried through here
#   present but unlisted a file added to a skill directory that no install ever
#                        delivers, so it works for the author and nowhere else
#
# Both are checked against `git ls-files`, which is the packaged set: package.json
# ships whole skill directories, so a tracked file is a shipped file.
#
# Two skills legitimately hold files no install delivers, and each exemption is a
# rule rather than a list of names, so a new file still has to justify itself.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/skill-files.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# SOURCE_ROOT is assigned after sourcing, because 05-config.sh declares it empty.
# skill_files() reads it for the globbed arms, so an empty root silently yields a
# short list -- which is why the count assertions below are not decoration.
# shellcheck disable=SC1090
source "$repo_root/installer/src/05-config.sh"
# shellcheck disable=SC1090
source "$repo_root/installer/src/50-manifest.sh"
SOURCE_ROOT="$repo_root"
SOURCE_VERSION='test'
REPO_REF='test'

# A file a skill directory may hold without an install delivering it. The reason
# is the point: without one, the file is a packaging bug, not an exemption.
unshipped_reason() { # <skill> <path> → prints the reason, or nothing
    case "$1/$2" in
        # Developer-facing documentation and the maintainer's own tooling. The
        # shipped set of planning is PACKAGE-MANIFEST.txt, diffed against this
        # list by planning/tests/test-installer-manifest.sh.
        planning/ARCHITECTURE.md | planning/MAINTAINER.md | planning/PACKAGE-MAP.tsv | planning/.gitignore)
            printf 'developer documentation, not part of the installed skill\n' ;;
        # The per-function sources and the compiler that turns them into the
        # shipped plan-*-lib.sh. The compiled libraries are listed; their inputs
        # are not, or every install would carry both copies.
        planning/scripts/lib/*.sh | planning/scripts/lib/*/*.sh | planning/scripts/build-plan-libs.sh)
            printf 'a compiler input; the compiled library is what ships\n' ;;
        # Example deviation files. The skill teaches the format; the examples are
        # this repo's own projects and mean nothing on another machine.
        project-specificies/*-deviations.md)
            printf 'an example of the format, specific to this repo\n' ;;
        *)
            # Everything else has to be named in the ratchet file, which is where
            # a decision that has not been made yet is parked.
            grep -qx "$2" "$ratchet" 2>/dev/null \
                && printf 'listed in %s pending a ship-or-not decision\n' "${ratchet##*/}" ;;
    esac
}

# The ratchet may only shrink: an entry leaves it by being registered for
# shipping. Comments and blanks are not entries.
ratchet="$repo_root/installer/unshipped-planning-files.txt"
[ -f "$ratchet" ] || t_fail "the unshipped register is missing: $ratchet"
ratchet_entries="$(grep -v '^#' "$ratchet" | grep -c . || true)"
t_assert_eq 'the unshipped register still holds entries' \
    "$([ "$ratchet_entries" -gt 0 ] && printf nonempty)" 'nonempty'
# A stale entry is as bad as a missing one: it says a decision is pending about a
# file that no longer exists, and it would mask a real omission if the name came
# back for something else.
stale=''
while IFS= read -r entry; do
    [ -n "$entry" ] || continue
    [ -e "$repo_root/planning/$entry" ] || stale="$stale $entry"
done < <(grep -v '^#' "$ratchet" | grep .)
t_assert_eq 'every entry in the unshipped register still exists' "${stale# }" ''

total_listed=0
for skill in "${SKILL_NAMES[@]}"; do
    skill_files "$skill" | sort > "$work/listed"
    ( cd "$repo_root/$skill" && git ls-files | sort ) > "$work/tracked"

    listed_count="$(grep -c . < "$work/listed" || true)"
    tracked_count="$(grep -c . < "$work/tracked" || true)"
    # A positive control on the harness itself: an empty list would satisfy the
    # "everything listed exists" check for every skill.
    [ "$listed_count" -gt 0 ] || t_fail "$skill: skill_files() listed nothing"
    [ "$tracked_count" -gt 0 ] || t_fail "$skill: the skill directory tracks nothing"
    total_listed=$((total_listed + listed_count))

    # Existence is asked of the filesystem, not of git ls-files: the installer
    # copies files, so an unstaged rename is exactly the case that must fail here.
    # (git ls-files still reported the old name, and this check passed on a
    # renamed schema until it was moved off the index.)
    absent=''
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        [ -e "$repo_root/$skill/$path" ] || absent="$absent $path"
    done < "$work/listed"
    t_assert_eq "$skill: every file skill_files() promises exists on disk" "${absent# }" ''

    # Anything tracked but unlisted must have a reason, and the reason is printed
    # with the failure so the next reader knows which of the two to fix.
    unexplained=''
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        [ -n "$(unshipped_reason "$skill" "$path")" ] || unexplained="$unexplained $path"
    done < <(comm -13 "$work/listed" "$work/tracked")
    t_assert_eq "$skill: every tracked file either ships or has a stated reason not to" \
        "${unexplained# }" ''
done

# The counts are a second control: a case arm that stopped matching would drop a
# skill's whole list, and each per-skill check above would still pass vacuously
# were it not for the guard, so pin the total the manifest is expected to carry.
t_assert_eq 'the manifest lists the expected number of files in total' \
    "$([ "$total_listed" -ge 150 ] && printf 'at least 150')" 'at least 150'

# ── the exemptions are exemptions, not a blanket ────────────────────────────
# A rule that matched everything would make the check above meaningless.
t_assert_eq 'an ordinary skill file is not exempt' \
    "$(unshipped_reason planning SKILL.md)" ''
t_assert_eq 'a new script under scripts/ is not exempt' \
    "$(unshipped_reason planning scripts/plan-new-thing.sh)" ''
t_assert_eq 'a new file in a register skill is not exempt' \
    "$(unshipped_reason todo schema.9.9.9.json)" ''
t_assert_contains 'a compiler input is exempt, and says why' 'compiled library' \
    "$(unshipped_reason planning scripts/lib/core/plan_die.sh)"

t_end
