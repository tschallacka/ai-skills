#!/usr/bin/env bash
# MODE: DEV
# test-rjq-in-membership.sh - rjq implements jq's IN/1 and IN/2 (T85), ported
# from jq's own builtin.jq: "does . equal any value the generator(s) produce."
# The prior trap this closes: rjq exited 5 with nothing on stdout, so a
# `$(rjq ...)` capture read as empty -- the same shape as a sound register --
# and reported every register sound without the check ever running.
set -euo pipefail
export LC_ALL=C

command -v rjq >/dev/null 2>&1 || {
    printf '%s\n' 'test-rjq-in-membership: SKIP (rjq is not on PATH; run ./setup-dev-env.sh)'
    exit 0
}

FAILED=0
fail() { printf 'test-rjq-in-membership: %s\n' "$1" >&2; FAILED=1; }

check() { # <description> <filter> <input> <expected stdout>
    local desc="$1" filter="$2" input="$3" expected="$4" out
    out="$(printf '%s' "$input" | rjq -c "$filter" 2>&1)" || {
        fail "$desc: rjq exited nonzero: $out"
        return
    }
    [ "$out" = "$expected" ] || fail "$desc: got '$out', wanted '$expected'"
}

check 'IN/1 true' 'IN(1,2,3)' '1' 'true'
check 'IN/1 false' 'IN(1,2,3)' '4' 'false'
check 'IN/1 against an empty generator is false, not an error' 'IN(empty)' '"z"' 'false'
check 'IN/2 true' 'IN((1,2); (2,3))' 'null' 'true'
check 'IN/2 false' 'IN((1,2); (3,4))' 'null' 'false'

# The register snippets both SKILL.md docs carry today deliberately avoid
# IN/1 (T85's own trap); this proves rjq still runs them cleanly, so landing
# IN/1 fixed the gap without moving the ground under the docs' own examples.
check 'index-based membership from bug-report/SKILL.md still runs clean' \
    '["a","b"] as $ids | (["blocking","major","minor","cosmetic"] | index("major")) != null' \
    'null' 'true'

[ "$FAILED" -eq 0 ] || exit 1
printf '%s\n' 'test-rjq-in-membership: PASS'
