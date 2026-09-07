#!/usr/bin/env bash
# MODE: DEV
# test-man-page-roff.sh — the shipped man pages carry well-formed roff escapes.
#
# B240: twelve lines of ai-text-editor.1 had lost their font escapes. A
# substitution pass over the file had interpreted "\f" and left a literal
# form-feed byte in its place, so "\fBcapabilities\fR" became
# <FF>Bcapabilities<FF>R and groff rendered it as the word "BcapabilitiesR".
# Nothing checked the man page's roff, so it rendered wrong silently through
# every release that shipped it.
#
# Two assertions, and the first is the one that bites: an escape that was
# dropped this way leaves a control byte behind, and no man page has a
# legitimate use for one. The second catches the opposite mangling — a "\f"
# that survived but names no font — and the third catches a font opened and
# never closed, which bleeds bold into the rest of the paragraph.
#
# `man --warnings` cannot see any of this: a form feed is legal roff, so the
# broken file rendered without a single warning. The source bytes are the
# only place the defect is visible.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

# Every tracked man page. benchmark/results holds immutable run evidence whose
# stray ".1" files are plan-document temporaries, not roff.
man_pages() {
    (cd "$repo_root" && git ls-files '*.1' \
        | awk '$0 !~ /^benchmark\/results\//')
}

pages="$(man_pages)"
[ -n "$pages" ] || t_fail 'no man pages found to check'

# ---- no control bytes -------------------------------------------------------
# A dropped escape leaves the character the escape named: \f becomes 0x0C,
# \b becomes 0x08. Newline is the line separator and tab is legitimate roff
# input, so those two are what a man page may contain; nothing else is.
for page in $pages; do
    stray="$(LC_ALL=C tr -dc '\000-\010\013-\037\177' \
        <"$repo_root/$page" | wc -c | tr -d ' ')"
    t_assert_eq "$page carries no control bytes" "$stray" 0
done

# ---- every \f names a font --------------------------------------------------
# Valid font escapes are \fB, \fI, \fR, \fP and the two-character \f(xx form.
for page in $pages; do
    bad="$(awk '{
        line = $0
        while (match(line, /\\f/)) {
            rest = substr(line, RSTART + 2)
            first = substr(rest, 1, 1)
            if (first != "B" && first != "I" && first != "R" \
                && first != "P" && first != "(") {
                print FILENAME ":" NR
            }
            line = rest
        }
    }' "$repo_root/$page" | wc -l | tr -d ' ')"
    t_assert_eq "$page has no \\f escape without a font name" "$bad" 0
done

# ---- fonts opened on a line are closed on it -------------------------------
# The house style in these pages is one font change per phrase, closed before
# the line ends; an unclosed \fB bleeds bold through everything after it.
for page in $pages; do
    unbalanced="$(awk '{
        opened = gsub(/\\fB/, "&") + gsub(/\\fI/, "&")
        closed = gsub(/\\fR/, "&") + gsub(/\\fP/, "&")
        if (opened != closed) { print FILENAME ":" NR }
    }' "$repo_root/$page" | wc -l | tr -d ' ')"
    t_assert_eq "$page closes every font it opens on the same line" \
        "$unbalanced" 0
done

t_end
