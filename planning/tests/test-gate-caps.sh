#!/usr/bin/env bash
# MODE: DEV
# test-gate-caps.sh — ratchet caps may shrink, never grow, without a
# deliberate override recorded in gate-caps.json.
#
# Usage: test-gate-caps.sh
#
# Reads the caps the ratchet tests actually use and compares them against
# gate-caps.json. A mismatch means someone raised (or lowered) a cap in the
# ratchet script without updating the approved baseline. Lowering is fine —
# update gate-caps.json to match. Raising requires justification in the
# commit message AND a matching gate-caps.json entry.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
caps_file="$repo_root/planning/gate-caps.json"

fail() { printf 'gate-caps: %s\n' "$1" >&2; exit 1; }

[ -f "$caps_file" ] || fail "gate-caps.json not found at $caps_file"

# ---- function-length cap ------------------------------------------------------
fl_cap="$(sed -n 's/^CAP=\([0-9]*\)$/\1/p' "$repo_root/planning/tests/test-function-length-ratchet.sh" | head -1)"
[ -n "$fl_cap" ] || fail "could not read CAP from test-function-length-ratchet.sh"

approved_fl="$(jq -r '.function_length_cap' "$caps_file")"
[ -n "$approved_fl" ] && [ "$approved_fl" != "null" ] || fail "function_length_cap missing from gate-caps.json"

if [ "$fl_cap" -gt "$approved_fl" ]; then
    fail "function-length cap raised from $approved_fl to $fl_cap — REQUIRES HUMAN APPROVAL; ask before proceeding"
fi

if [ "$fl_cap" -lt "$approved_fl" ]; then
    printf 'gate-caps: function-length cap lowered from %s to %s; update gate-caps.json\n' \
        "$approved_fl" "$fl_cap" >&2
fi

# ---- duplication caps ----------------------------------------------------------
dup_script="$repo_root/planning/tests/test-duplication-ratchet.sh"
dup_failed=0
while IFS= read -r line; do
    label="$(printf '%s' "$line" | sed "s/^check_cap '//; s/'.*//")"
    cap="$(printf '%s' "$line" | sed "s/^check_cap '[^']*' \([0-9]*\).*/\1/")"
    [ -n "$label" ] && [ -n "$cap" ] || continue
    approved="$(jq -r --arg k "$label" '.duplication_caps[$k] // ""' "$caps_file")"
    if [ -z "$approved" ] || [ "$approved" = "null" ]; then
        printf 'gate-caps: cap '"'"'%s'"'"' not found in gate-caps.json\n' "$label" >&2
        dup_failed=1
        continue
    fi
    if [ "$cap" -gt "$approved" ]; then
        printf 'gate-caps: duplication cap '"'"'%s'"'"' raised from %s to %s — REQUIRES HUMAN APPROVAL; ask before proceeding\n' \
            "$label" "$approved" "$cap" >&2
        dup_failed=1
    fi
    if [ "$cap" -lt "$approved" ]; then
        printf 'gate-caps: duplication cap '"'"'%s'"'"' lowered from %s to %s; update gate-caps.json\n' \
            "$label" "$approved" "$cap" >&2
    fi
done < <(grep "^check_cap '" "$dup_script")

if [ "$dup_failed" -ne 0 ]; then
    fail "duplication ratchet caps diverged from gate-caps.json"
fi

printf '%s\n' 'test-gate-caps: PASS'
