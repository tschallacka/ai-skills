#!/usr/bin/env bash
# MODE: DEV
# test-mods-package.sh - the mods ship with the skills they serve.
#
# Three things must agree: the boards the release collects (installer/
# build-release.sh's MODS_BOARDS), the boards the installer copies onto a Claude
# root (plugins::MODS in src/installer/src/plugins.rs), and the mods that exist on
# disk. And every board must have at least one tracked file, or the tarball has
# nothing to carry for it.
set -euo pipefail
export LC_ALL=C

root="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$root/planning/tests/lib-test.sh"
t_begin

release_boards="$(sed -n 's/^MODS_BOARDS="\(.*\)"$/\1/p' "$root/installer/build-release.sh" | tr ' ' '\n' | sort)"
# A board name opens its tuple on the line with the "(" (rustfmt may put the name
# on the line below it), so a name is read where that line opens a tuple.
installer_boards="$(awk '/^pub const MODS/{f=1; next} f && /^\];/{f=0} f {
        if (match($0, /"[a-z-]+",/) && ($0 ~ /\(/ || prev ~ /\($/)) {
            print substr($0, RSTART + 1, RLENGTH - 3)
        }
        prev = $0
    }' "$root/src/installer/src/plugins.rs" | sort)"
t_assert_eq 'the release collects the same boards the installer copies' \
    "$installer_boards" "$release_boards"

while IFS= read -r board; do
    [ -n "$board" ] || continue
    [ -d "$root/mods/$board" ] || t_fail "$board is listed but mods/$board does not exist"
    tracked="$(git -C "$root" ls-files "mods/$board" | wc -l | tr -d ' ')"
    [ "$tracked" -gt 0 ] || t_fail "$board has no tracked files, so the package would carry none"
done <<EOF
$release_boards
EOF

# package.json's files list is what npm ships, and it overrides .npmignore, so
# each board must be named there (loading and turn-cue are deliberately not).
while IFS= read -r board; do
    [ -n "$board" ] || continue
    t_assert_eq "package.json ships mods/$board" \
        "$(grep -c "\"mods/$board\"" "$root/package.json")" '1'
done <<EOF
$release_boards
EOF

t_end
