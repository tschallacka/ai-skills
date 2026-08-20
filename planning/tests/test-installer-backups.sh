#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# test-installer-backups.sh — an install never destroys a file the user edited,
# and never leaves a backup for one it wrote itself.
#
# The bug this pins: the installer decided per SKILL, from the .version marker.
# When that marker differed it replaced every file "without backups" -- including
# files the user had edited, with no git and no undo behind them. A per-file
# digest recorded at install time makes the distinction the marker cannot:
# "we wrote this and nobody touched it" versus "the user changed it".

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-backups.XXXXXX")"
trap 'rm -rf "$work"' EXIT

target="$work/root"
skill=planning
installed="$target/$skill"

# The skill install succeeds and then the optional agent-permissions step asks
# for interactive input and exits non-zero. That is not this test's concern, so
# tolerate the status and assert the files landed instead.
install_once() {
    ( cd "$repo_root" && ./install.sh --skill "$skill" --target "$target" --yes ) \
        >/dev/null 2>&1 || true
    [ -f "$installed/SKILL.md" ] || t_fail 'the install did not place SKILL.md'
}

backups_for() { # <basename> -> count
    find "$target" -name ".$1.*.back" 2>/dev/null | { grep -c . || true; }
}

install_once
[ -f "$installed/.filehashes" ] || t_fail 'the install recorded no per-file digests'
t_assert_eq 'a fresh install leaves no backup' "$(backups_for 'SKILL.md')" 0

# Reinstalling an untouched tree must not litter it.
install_once
t_assert_eq 'reinstalling an untouched tree leaves no backup' "$(backups_for 'SKILL.md')" 0

# A file the user edited is preserved, under a dotfile name so a plain grep of
# the installed tree does not pick it up.
printf '\n<!-- a local edit -->\n' >> "$installed/SKILL.md"
install_once
t_assert_eq 'an edited file is backed up' "$(backups_for 'SKILL.md')" 1
backup="$(find "$target" -name '.SKILL.md.*.back' | head -1)"
[ -n "$backup" ] || t_fail 'no backup path to inspect'
if [ -n "$backup" ]; then
    grep -Fq 'a local edit' "$backup" || t_fail 'the backup does not contain the edit it was taken for'
    case "$(basename "$backup")" in
        .SKILL.md.*.back) ;;
        *) t_fail "backup is not named .<file>.<timestamp>.back: $(basename "$backup")" ;;
    esac
fi

# The case the digests exist for: content we shipped in an older version, which
# the user never touched, is replaced silently -- while an edit in the same run
# is still preserved. Simulated by rewriting a file and recording that content as
# ours, which is what an older install would have left behind.
printf 'CONTENT FROM AN OLDER VERSION\n' > "$installed/ROLES.md"
# Must match content_digest in installer/src/60-install.sh: this stands in for
# what an older install would have recorded.
older="$(cksum "$installed/ROLES.md" | awk '{print $1 "-" $2}')"
awk -v d="$older" '$2 == "ROLES.md" { print d, $2; next } { print }' \
    "$installed/.filehashes" > "$work/fh" && mv "$work/fh" "$installed/.filehashes"
printf '\n<!-- another local edit -->\n' >> "$installed/REVIEWER.md"
install_once
t_assert_eq 'our own older content is replaced without a backup' "$(backups_for 'ROLES.md')" 0
t_assert_eq 'an edit in the same run is still backed up' "$(backups_for 'REVIEWER.md')" 1
grep -Fq 'CONTENT FROM AN OLDER VERSION' "$installed/ROLES.md" \
    && t_fail 'the older content was not replaced'

# ---- inside a git work tree, history is the recovery path -------------------
# A .back file there is clutter: the user already has git. The install must still
# SAY what it replaced, so an uncommitted edit is lost visibly rather than
# silently (CODE-CONTRACTS.md contract 9).
git_target="$work/git-root"
mkdir -p "$git_target"
git init -q "$git_target"
( cd "$repo_root" && ./install.sh --skill "$skill" --target "$git_target" --yes ) \
    >/dev/null 2>&1 || true
[ -f "$git_target/$skill/SKILL.md" ] || t_fail 'the install did not populate the git-tracked target'
printf '\n<!-- an edit inside a work tree -->\n' >> "$git_target/$skill/SKILL.md"
out="$( ( cd "$repo_root" && ./install.sh --skill "$skill" --target "$git_target" --yes ) 2>&1 || true )"
t_assert_eq 'no backup file is written inside a git work tree' \
    "$(find "$git_target" -name '.*.back' 2>/dev/null | { grep -c . || true; })" 0
t_assert_contains 'the replacement is reported instead' 'recoverable with git' "$out"

t_end
