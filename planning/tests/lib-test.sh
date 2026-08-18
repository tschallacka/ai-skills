#!/usr/bin/env bash
# lib-test — portability shims for the test suites.
#
# Usage: sourced by a test, never executed.
#   t_sed_i <sed-script> <file>     in-place edit, no GNU -i
#   t_sed_insert_before <re> <text> <file>
#   t_stat_mode <file>              octal mode, GNU or BSD stat
#   t_unique_suffix                 a unique token, no `date +%N`
#   t_copy_tree <src> <dst>         contents incl. dotfiles, no `cp -R src/.`
#
# The tests run on the same bash 3.2 + BSD floor as the scripts (CI runs the
# suite on macos), so they need the same shims. See PORTABILITY.md; the rule ids
# in the markers below index into it.
set -euo pipefail

# PORTABILITY(sed-inplace): BSD sed requires a suffix argument for -i, so it
# consumes the script as the suffix and then fails.
t_sed_i() {
    local script="$1" file="$2" temporary
    temporary="$(mktemp "${TMPDIR:-/tmp}/t-sed.XXXXXX")"
    sed "$script" "$file" > "$temporary"
    mv -f "$temporary" "$file"
}

# BSD sed rejects `i text` on one line; awk sidesteps the dialect entirely.
t_sed_insert_before() {
    local pattern="$1" text="$2" file="$3" temporary
    temporary="$(mktemp "${TMPDIR:-/tmp}/t-ins.XXXXXX")"
    awk -v pat="$pattern" -v ins="$text" '
        $0 ~ pat { print ins }
        { print }
    ' "$file" > "$temporary"
    mv -f "$temporary" "$file"
}

# PORTABILITY(stat-format): probe once; GNU takes -c, BSD takes -f.
if stat -c '%a' . >/dev/null 2>&1; then
    t_stat_mode() { stat -c '%a' "$1"; }
else
    t_stat_mode() { stat -f '%Lp' "$1"; }
fi

# PORTABILITY(date-nanoseconds): BSD date has no %N and emits a literal "N".
t_unique_suffix() {
    printf '%s_%s%s' "$$" "${RANDOM}" "${RANDOM}"
}

# PORTABILITY(cp-dot-source): a source path ending in `/.` is unspecified.
# `cp -R src/. dst` creates dst implicitly and tar does not, so mkdir first or
# this is not a faithful replacement.
t_copy_tree() {
    mkdir -p "$2"
    ( cd "$1" && tar cf - . ) | ( cd "$2" && tar xf - )
}
