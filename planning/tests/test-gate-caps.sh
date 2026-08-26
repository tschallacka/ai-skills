#!/usr/bin/env bash
# MODE: DEV
# test-gate-caps.sh — ratchet caps only ever go down, never up.
#
# Usage: test-gate-caps.sh          verify gates
#        test-gate-caps.sh --clamp  apply observed lows to tests + json
#
# Three enforced properties:
#   1. test caps == gate-caps.json exactly (a change in one without the
#      other is divergence, caught in whichever direction);
#   2. no cap may exceed its value in git history — a coordinated raise of
#      test + json in one commit is still a raise, still a failure. There
#      is no approval path: reduce the count instead;
#   3. when a count drops below its cap, `--clamp` writes the observed low
#      into the ratchet test and gate-caps.json mechanically, so the upper
#      limit is always the current low (update MAINTAINER.md section 3 in
#      the same commit — clamp prints the reminder).
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
caps_file="$repo_root/planning/gate-caps.json"
fl_test="$repo_root/planning/tests/test-function-length-ratchet.sh"
dup_test="$repo_root/planning/tests/test-duplication-ratchet.sh"

fail() { printf 'gate-caps: %s\n' "$1" >&2; exit 1; }
[ -f "$caps_file" ] || fail "gate-caps.json not found at $caps_file"

# ---- --clamp: apply observed lows ---------------------------------------------
if [ "${1:-}" = "--clamp" ]; then
    lowered=0; growth=0
    dup_out="$(bash "$dup_test" 2>&1 || true)"
    while IFS= read -r msg; do
        [ -n "$msg" ] || continue
        line="$(printf '%s' "$msg" | sed -E 's/^.*ratchet: (.*) is down to ([0-9]+) \(cap ([0-9]+)\).*$/\1|->\2|->\3/')"
        case "$line" in
            *'|->'*) : ;;
            *) continue ;;
        esac
        label="${line%%|->*}"
        rest="${line#*|->}"
        new="${rest%%|->*}"; old="${rest#*|->}"
        ln="$(grep -nF -- "$label" "$dup_test" | head -1 | cut -d: -f1)"
        [ -n "$ln" ] || fail "clamp: cannot find check_cap line for '$label'"
        sed_line="$(sed -n "${ln}p" "$dup_test")"
        case "$sed_line" in
            *" $old "*) t_sed_i "${ln}s/ $old / $new /" "$dup_test" ;;
            *) fail "clamp: cap $old not found on line $ln of test-duplication-ratchet.sh" ;;
        esac
        tmp_json="$(mktemp "${TMPDIR:-/tmp}/gate-caps.XXXXXX")"
        jq --arg k "$label" --argjson v "$new" '.duplication_caps[$k] = $v' \
            "$caps_file" > "$tmp_json" && mv "$tmp_json" "$caps_file"
        printf 'gate-caps: lowered %s %s -> %s\n' "$label" "$old" "$new"
        lowered=1
    done <<< "$(printf '%s\n' "$dup_out" | grep 'is down to')"
    if printf '%s\n' "$dup_out" | grep -q 'grew to'; then
        printf 'gate-caps: counts GREW above their caps — clamp applied the lowerings above but the growth needs the duplication removed:\n' >&2
        printf '%s\n' "$dup_out" | grep 'grew to' >&2
        growth=1
    fi

    fl_out="$(bash "$fl_test" 2>&1 || true)"
    fl_line="$(printf '%s\n' "$fl_out" | sed -n 's/^\([0-9][0-9]*\) function(s) exceed the 40-line cap (cap \([0-9][0-9]*\)): lower the cap in this commit.*$/\1|->\2/p' | head -1)"
    if [ -n "$fl_line" ]; then
        new="${fl_line%%|->*}"; old="${fl_line#*|->}"
        t_sed_i "s/^CAP=$old\$/CAP=$new/" "$fl_test"
        tmp_json="$(mktemp "${TMPDIR:-/tmp}/gate-caps.XXXXXX")"
        jq --argjson v "$new" '.function_length_cap = $v' \
            "$caps_file" > "$tmp_json" && mv "$tmp_json" "$caps_file"
        printf 'gate-caps: lowered function_length_cap %s -> %s\n' "$old" "$new"
        lowered=1
    fi
    if printf '%s\n' "$fl_out" | grep -q 'New over-cap code must be split'; then
        printf 'gate-caps: over-cap functions GREW above the cap — clamp applied any lowerings above but the growth needs the code split\n' >&2
        growth=1
    fi

    [ "$lowered" = 1 ] && printf '%s\n' 'gate-caps: reminder — update MAINTAINER.md section 3 rows in the SAME commit'
    [ "$growth" = 0 ] || exit 1
    exec bash "$0"
fi

# ---- property 1: test caps == json --------------------------------------------
fl_cap="$(sed -n 's/^CAP=\([0-9]*\)$/\1/p' "$fl_test" | head -1)"
[ -n "$fl_cap" ] || fail "could not read CAP from test-function-length-ratchet.sh"

approved_fl="$(jq -r '.function_length_cap' "$caps_file")"
[ -n "$approved_fl" ] && [ "$approved_fl" != "null" ] || fail "function_length_cap missing from gate-caps.json"

if [ "$fl_cap" -ne "$approved_fl" ]; then
    printf 'gate-caps: function-length cap is %s in the test but %s in gate-caps.json; they must match in the same commit (run test-gate-caps.sh --clamp to apply the lower one)\n' \
        "$fl_cap" "$approved_fl" >&2
    fail "function-length cap diverged from gate-caps.json"
fi

dup_failed=0
while IFS= read -r line; do
    label="$(printf '%s' "$line" | sed -E "s/^check_cap (['\"])(.*)\\1 [0-9]*.*$/\\2/")"
    cap="$(printf '%s' "$line" | sed -E "s/^check_cap (['\"]).*\\1 ([0-9]*).*$/\\2/")"
    [ -n "$label" ] && [ -n "$cap" ] || continue
    approved="$(jq -r --arg k "$label" '.duplication_caps[$k] // ""' "$caps_file")"
    if [ -z "$approved" ] || [ "$approved" = "null" ]; then
        printf 'gate-caps: cap %s not found in gate-caps.json\n' "$label" >&2
        dup_failed=1
        continue
    fi
    if [ "$cap" -ne "$approved" ]; then
        printf 'gate-caps: cap %s is %s in the test but %s in gate-caps.json; they must match in the same commit (run test-gate-caps.sh --clamp to apply the lower one)\n' \
            "$label" "$cap" "$approved" >&2
        dup_failed=1
    fi
done < <(grep -E '^check_cap (["'"'"'])' "$dup_test")
[ "$dup_failed" -eq 0 ] || fail "duplication ratchet caps diverged from gate-caps.json"

# ---- property 2: never up, checked against git history ------------------------
# Baseline = the newest recorded state that predates the working changes:
# uncommitted edits compare against HEAD; a clean tree (CI at the change
# commit) compares against the previous commit that touched the file. A
# coordinated test+json raise passes property 1 and dies here.
baseline_file=""
if git -C "$repo_root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    tmp_base="$(mktemp "${TMPDIR:-/tmp}/gate-caps-base.XXXXXX")"
    if git -C "$repo_root" show HEAD:planning/gate-caps.json > "$tmp_base" 2>/dev/null; then
        if ! diff -q "$tmp_base" "$caps_file" >/dev/null 2>&1; then
            baseline_file="$tmp_base"          # uncommitted changes vs HEAD
        else
            prev="$(git -C "$repo_root" log --skip=1 -1 --format=%H -- planning/gate-caps.json 2>/dev/null || true)"
            if [ -n "$prev" ] && git -C "$repo_root" show "$prev:planning/gate-caps.json" > "$tmp_base" 2>/dev/null; then
                baseline_file="$tmp_base"      # HEAD may itself be the raise
            else
                rm -f "$tmp_base"
            fi
        fi
    else
        rm -f "$tmp_base"
    fi
fi

if [ -n "$baseline_file" ]; then
    rose=0
    base_fl="$(jq -r '.function_length_cap // empty' "$baseline_file")"
    if [ -n "$base_fl" ] && [ "$approved_fl" -gt "$base_fl" ]; then
        printf 'gate-caps: function_length_cap went UP from %s to %s — caps only ever go down; reduce the count instead\n' \
            "$base_fl" "$approved_fl" >&2
        rose=1
    fi
    while IFS= read -r key; do
        [ -n "$key" ] || continue
        cur="$(jq -r --arg k "$key" '.duplication_caps[$k] // empty' "$caps_file")"
        base="$(jq -r --arg k "$key" '.duplication_caps[$k] // empty' "$baseline_file")"
        [ -n "$cur" ] && [ -n "$base" ] || continue
        if [ "$cur" -gt "$base" ]; then
            printf 'gate-caps: cap %s went UP from %s to %s — caps only ever go down; reduce the count instead\n' \
                "$key" "$base" "$cur" >&2
            rose=1
        fi
    done < <(jq -r '.duplication_caps | keys[]' "$caps_file")
    [ "$rose" -eq 0 ] || { rm -f "$baseline_file"; fail "a cap rose against its recorded history"; }
    rm -f "$baseline_file"
fi

printf '%s\n' 'test-gate-caps: PASS'
