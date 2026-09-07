#!/usr/bin/env bash
# MODE: DEV
# test-function-length-ratchet — CODE-STYLE.md's 40-line function cap, ratcheted.
#
# Usage: test-function-length-ratchet.sh
#
# CODE-STYLE.md section on size limits caps a function at 40 lines ("extract a
# helper"), but nothing enforced it: launch_agent reached 93 lines before T33
# named it. Splitting all 67 then-over-cap functions in one sweep is not the
# move — several are deliberate data tables or recently reviewed gates — so the
# debt is ratcheted instead: the count of over-cap functions may shrink, never
# grow. A new over-cap function fails here; pay the cap at the moment you add
# the code. On a genuine split, lower the cap in the same commit. Never raise it.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

note_fail() { printf 'function-length-ratchet: %s\n' "$1" >&2; t_record "$1"; }

# Over-cap functions in one function's worth of awk, so the scoped mode below
# and the whole-tree ratchet measure shape identically. Prints "<len> <line>
# <name>" per over-cap function in the file named by $1.
over_cap_in() {
    awk '/^[a-zA-Z_][a-zA-Z0-9_]*\(\) *\{[[:space:]]*(#.*)?$/{start=NR; name=$0} start && /^\}$/{if (NR-start+1>40) print NR-start+1, start, name; start=""}' "$1"
}

# `--files [--base REF] <path>...` applies the ratchet's SPIRIT to one change
# rather than the tree: a function that is NEWLY over the 40-line cap fails. It
# says nothing about the tree-wide count, which is a global property no per-file
# run can evaluate; CI keeps that.
#
# Comparing against the base matters, and two drafts of this got it wrong in the
# same direction -- too strict, which is how a gate teaches people to bypass it.
#
# The first flagged every over-cap function in a touched file, so it refused any
# edit to a file that already contained one (70-permissions.sh carries three
# from before this mode existed).
#
# The second also failed an already over-cap function that merely GREW, and that
# fired immediately on a legitimate change: skill_files() is a 556-line
# hand-maintained data list -- installer/src/50-manifest.sh says so, and the
# duplication against PACKAGE-MANIFEST.tsv IS the cross-check -- so adding one
# manifest row grew it by a line and failed the push. Growth of a function that
# was already over cap is not this gate's business: the tree-wide ratchet
# governs the COUNT, it may not grow, and a data table gaining a row does not
# change it. Only crossing the cap is new debt, so only that is reported.
#
# With no --base (or an unknown ref) there is nothing to compare against, so
# every over-cap function in the named files is reported and the caller decides.
if [ "${1:-}" = "--files" ]; then
    shift
    base_ref=""
    if [ "${1:-}" = "--base" ]; then
        base_ref="${2:-}"
        shift 2
    fi
    [ "$#" -gt 0 ] || { printf 'function-length-ratchet: --files needs at least one path\n' >&2; exit 64; }

    # "<name> <length>" per over-cap function, so a length can be looked up by
    # name on either side of the comparison.
    lengths_for() { # <file-on-disk-or-empty>
        [ -n "$1" ] && [ -f "$1" ] || return 0
        over_cap_in "$1" | while read -r len _line name_; do
            [ -n "$len" ] || continue
            printf '%s %s\n' "${name_%%(*}" "$len"
        done
    }

    scoped_over=""
    base_tmp="$(mktemp "${TMPDIR:-/tmp}/cap-base.XXXXXX")"
    trap 'rm -f "$base_tmp"' EXIT
    for arg in "$@"; do
        target="$root/$arg"
        [ -f "$target" ] || target="$arg"
        [ -f "$target" ] || continue

        : > "$base_tmp"
        if [ -n "$base_ref" ]; then
            git -C "$root" show "$base_ref:$arg" > "$base_tmp" 2>/dev/null || : > "$base_tmp"
        fi
        base_lengths="$(lengths_for "$base_tmp")"

        while read -r name_ len; do
            [ -n "$name_" ] || continue
            was=""
            while read -r bname blen; do
                [ "$bname" = "$name_" ] && was="$blen"
            done <<BASE_EOF
$base_lengths
BASE_EOF
            # Absent from the base's over-cap set means it crossed the cap in
            # this change: either it is new, or it was under 40 lines and now is
            # not. Both are new debt. A function already over cap is left alone
            # even if it grew -- see the header for why that check had to go.
            if [ -z "$was" ]; then
                scoped_over="$scoped_over  $arg: $name_() is $len lines, over the 40-line cap
"
            fi
        done <<NOW_EOF
$(lengths_for "$target")
NOW_EOF
    done

    if [ -n "$scoped_over" ]; then
        printf 'function-length-ratchet: this change puts a function over the 40-line cap:\n' >&2
        printf '%s' "$scoped_over" >&2
        printf 'function-length-ratchet: split it; CODE-STYLE.md caps a function at 40 lines\n' >&2
        exit 1
    fi
    printf '%s\n' 'test-function-length-ratchet: PASS (changed files, nothing newly over the cap)'
    exit 0
fi

CAP=58
count=0
worst=""
for f in $(git -C "$root" ls-files '*.sh' | grep -v '^benchmark/results/'); do
    # Deleted tracked files remain in git ls-files until the change is committed;
    # they are no longer code whose function length can be ratcheted.
    [ -f "$root/$f" ] || continue
    # A function runs from its `name() {` line to the first column-0 closing
    # brace; that is the same convention test-duplication-ratchet.sh uses to
    # extract functions, and CODE-STYLE.md section 12 forbids nothing here:
    # this is measuring shape, not parsing semantics.
    while read -r len line name_; do
        [ -n "$len" ] || continue
        count=$((count + 1))
        worst="$worst$len $f:$line $name_
"
    done <<EOF
$(over_cap_in "$root/$f")
EOF
done

if [ "$count" -gt "$CAP" ]; then
    note_fail "$count function(s) exceed the 40-line cap (cap $CAP). New over-cap code must be split; caps only ever go down."
fi
if [ "$count" -lt "$CAP" ]; then
    note_fail "$count function(s) exceed the 40-line cap (cap $CAP): lower the cap in this commit (tests/test-gate-caps.sh --clamp applies the low mechanically)"
fi

# Positive control: if the counter broke to zero it would agree with any cap.
# launch_agent's split (T33) is the most recent reduction; if every named site
# below disappears the cap must come down with them.
if [ "$count" -eq 0 ]; then
    note_fail "counter reports zero over-cap functions; either the cap was earned or the counter is broken"
fi

[ "$(t_failures)" -eq 0 ] || {
    printf '%s\n' "current over-cap sites:" >&2
    printf '%s' "$worst" >&2
    exit 1
}
printf '%s\n' 'test-function-length-ratchet: PASS'
