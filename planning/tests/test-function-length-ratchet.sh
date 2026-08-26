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

CAP=66
count=0
worst=""
for f in $(git -C "$root" ls-files '*.sh' | grep -v '^benchmark/results/'); do
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
$(awk '/^[a-zA-Z_][a-zA-Z0-9_]*\(\) *\{/{start=NR; name=$0} start && /^\}$/{if (NR-start+1>40) print NR-start+1, start, name; start=""}' "$root/$f")
EOF
done

if [ "$count" -gt "$CAP" ]; then
    note_fail "$count function(s) exceed the 40-line cap (cap $CAP). New over-cap code must be split. Raising the cap requires human approval."
fi
if [ "$count" -lt "$CAP" ]; then
    note_fail "$count function(s) exceed the 40-line cap (cap $CAP): lower the cap in this commit"
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
