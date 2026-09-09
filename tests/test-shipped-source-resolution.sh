#!/usr/bin/env bash
# MODE: DEV
# test-shipped-source-resolution.sh — a shipped script's `source` targets ship
# with it.
#
# B108: register-read.sh (MODE: PROD, in the planning prod arm) sources
# register-lib.sh (MODE: DEV, in the dev arm only) at a fixed sibling path.
# test-mode-markers.sh and test-skill-files-manifest.sh both pass on that pair
# individually -- each file's own marker matches its own arm -- so neither
# checks whether the PAIR is installable together. This is the missing check
# (B111): resolve every prod-shipped script's sibling `source` target against
# that skill's own prod arm, and fail if the target is not there.
#
# Scope, deliberately narrow: only the two forms every shipped source line
# actually uses --
#   source "$script_dir/NAME"
#   source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/NAME"
# -- are sibling-relative and resolvable without evaluating the script. A
# fully dynamic target (source "$file", source "$variables_file") cannot be
# resolved statically and is not this check's job; grep excludes both forms
# from the candidate lines below rather than guessing at them.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

# shellcheck disable=SC1090
source "$repo_root/installer/src/05-config.sh"
# shellcheck disable=SC1090
source "$repo_root/installer/src/50-manifest.sh"
SOURCE_ROOT="$repo_root"
SOURCE_VERSION='test'
REPO_REF='test'

checked=0
for skill in "${SKILL_NAMES[@]}"; do
    listed="$(skill_files "$skill" prod)"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        case "$relative" in *.sh) ;; *) continue ;; esac
        path="$repo_root/$skill/$relative"
        [ -f "$path" ] || continue
        checked=$((checked + 1))
        dir="$(dirname "$relative")"
        while IFS= read -r target; do
            [ -n "$target" ] || continue
            case "$dir" in
                .) sibling="$target" ;;
                *) sibling="$dir/$target" ;;
            esac
            if ! printf '%s\n' "$listed" | grep -cx "$sibling" >/dev/null; then
                t_fail "$skill/$relative sources $target, which is not in the $skill prod arm ($sibling)"
            fi
        done < <(grep -oE 'source "(\$script_dir|\$\(cd "\$\(dirname "\$\{BASH_SOURCE\[0\]\}"\)" && pwd\))/[A-Za-z0-9_.-]+"' "$path" \
            | sed -E 's#.*/([A-Za-z0-9_.-]+)"$#\1#' || true)
    done <<EOF
$listed
EOF
done

# A positive control on the harness itself: zero scripts checked would satisfy
# every assertion above vacuously.
t_assert_eq 'at least one shipped script with a sibling source line was checked' \
    "$([ "$checked" -gt 0 ] && printf yes)" yes
printf '%s: resolved sibling source targets across %s shipped script(s)\n' \
    "${0##*/}" "$checked" >&2

t_end
