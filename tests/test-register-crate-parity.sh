#!/usr/bin/env bash
# MODE: DEV
# test-register-crate-parity.sh - the two register crates share two files
# VERBATIM, and this is what keeps them shared.
#
# src/bug-report and src/todo are deliberately separate crates: the defect
# register and the work queue do not share their rules, and the one shell library
# that pretended they did grew a defect at every place it pretended (B102 among
# them). What they DO share is machinery that knows nothing about either
# register -- argument parsing and the clock -- and those files are copies.
#
# A copy that is allowed to drift is worse than either a shared crate or two
# honest implementations: a fix lands in one and not the other, and nothing says
# so. So the copies are pinned byte for byte here. Diverging them on purpose is
# allowed -- delete the pin and say why in the same commit -- but diverging them
# by forgetting is not.
#
# Two copies rather than a third crate they both depend on: a shared crate is
# the right shape once the shared surface is worth versioning, and this is 250
# lines of argument parsing and a clock. The pin below is what makes the copy
# safe in the meantime, and it is what has to be deleted -- deliberately, in the
# same commit -- if the two are ever meant to differ.

set -euo pipefail
export LC_ALL=C

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$root/planning/tests/lib-test.sh"
t_begin

shared="cli.rs clock.rs"

for file in $shared; do
    left="$root/src/bug-report/src/$file"
    right="$root/src/todo/src/$file"

    [ -f "$left" ] || t_fail "src/bug-report/src/$file is missing"
    [ -f "$right" ] || t_fail "src/todo/src/$file is missing"
    [ -f "$left" ] && [ -f "$right" ] || continue

    if cmp -s "$left" "$right"; then
        printf 'ok: src/{bug-report,todo}/src/%s are byte-identical\n' "$file"
    else
        t_fail "$(printf 'src/bug-report/src/%s and src/todo/src/%s have drifted:\n%s' \
            "$file" "$file" "$(diff "$left" "$right" | sed 's/^/    /' | head -20)")"
    fi
done

# A shared file must not name either register, or it is not shared machinery --
# it is one crate's code living in the other's copy, and the next rule change
# will have to be made twice.
for file in $shared; do
    target="$root/src/todo/src/$file"
    [ -f "$target" ] || continue
    leaked="$(awk '
        /^[[:space:]]*\/\// { next }               # comments may name both
        /Bug|Severity|bugs|BUGS\.json|Task|tasks|TODO\.json/ { print FNR ": " $0 }
    ' "$target" | head -5)"
    if [ -n "$leaked" ]; then
        t_fail "$(printf 'src/todo/src/%s names a register in its code, so it is not shared machinery:\n%s' \
            "$file" "$(printf '%s\n' "$leaked" | sed 's/^/    /')")"
    else
        printf 'ok: %s names neither register in its code\n' "$file"
    fi
done

t_end
