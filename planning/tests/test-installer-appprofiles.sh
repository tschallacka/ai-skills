#!/usr/bin/env bash
# MODE: DEV
# test-installer-appprofiles.sh — an interactive-shell install places its
# vendor-shipped TUI app profiles at the stable, agent-facing XDG location.
#
# These are read-only reference docs (interactive-shell/appprofiles/*.md),
# installed once regardless of how many agent roots were selected, and
# reinstalled (overwritten) every run so an updated profile always replaces
# an older one already on disk.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-appprofiles.XXXXXX")"
trap 'rm -rf "$work"' EXIT

install_into() { # <home> <target-subpath>
    local home="$1" target="$2"
    env -u XDG_CONFIG_HOME HOME="$home" "$BASH" "$installer" --skill interactive-shell \
        --target "$home/$target" --yes >"$work/out" 2>&1 </dev/null
}

home="$work/home"
rc=0
install_into "$home" .claude/skills || rc=$?
t_assert_eq 'an interactive-shell install installs cleanly' "$rc" '0'

destination="$home/.config/tsch-ai-skills/appprofiles"
t_assert_eq 'FORMAT.md landed at the stable location' \
    "$([ -f "$destination/FORMAT.md" ] && printf yes || printf no)" 'yes'
t_assert_eq 'mc.md landed at the stable location' \
    "$([ -f "$destination/mc.md" ] && printf yes || printf no)" 'yes'
t_assert_eq 'every shipped profile landed, none missing' \
    "$(cmp -s <(cd "$repo_root/interactive-shell/appprofiles" && ls -- *.md | sort) \
              <(cd "$destination" && ls -- *.md | sort) && printf same || printf different)" 'same'
t_assert_eq 'the delivered mc.md matches the shipped source' \
    "$(cmp -s "$repo_root/interactive-shell/appprofiles/mc.md" "$destination/mc.md" && printf same)" 'same'

# A stale/wrong file already at the destination must be overwritten by a
# fresh install, not left in place -- this is the "any new formats get
# installed/updated there too" requirement. Seeded at the destination
# directly (never touching the repo's own tracked source) so this cannot
# race a concurrent edit to the shipped profiles.
printf 'stale content from a previous version\n' > "$destination/mc.md"
rc=0
install_into "$home" .claude/skills || rc=$?
t_assert_eq 'a second install run installs cleanly' "$rc" '0'
t_assert_eq 'the stale destination copy was overwritten with the current shipped content' \
    "$(cmp -s "$repo_root/interactive-shell/appprofiles/mc.md" "$destination/mc.md" && printf same || printf different)" 'same'

t_end
