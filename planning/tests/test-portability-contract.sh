#!/usr/bin/env bash
# test-portability-contract — the portability catalogue stays true, and a gotcha
# caught once stays caught.
#
# Usage: test-portability-contract.sh
#
# Four assertions:
#   1. PORTABILITY.md matches what generate-portability.sh produces.
#   2. Every `# PORTABILITY(<id>)` marker names a rule in portability-rules.json.
#   3. No untagged `# PORTABILITY:` markers remain (they cannot be indexed).
#   4. No tracked script contains a detectable banned construct, unless the file
#      is allowlisted for that rule.
#
# Assertion 4 is the one that saves the next agent from rediscovering a trap in
# an unrelated file.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
rules="$repo_root/portability-rules.json"
generator="$repo_root/generate-portability.sh"

fail=0
note_fail() { printf 'portability: %s\n' "$1" >&2; fail=1; }

command -v jq >/dev/null 2>&1 || {
    printf 'portability: UNCONFIGURED (jq)\n' >&2
    exit 64
}
[ -f "$rules" ] || { note_fail "missing $rules"; exit 1; }
[ -x "$generator" ] || { note_fail "missing or non-executable $generator"; exit 1; }

# Files that legitimately contain a pattern: the registry names every construct,
# the catalogue publishes them, and plan-map-lib.sh is the replacement for one.
in_allowlist() {
    case "$1" in
        ./generate-portability.sh|./planning/tests/test-portability-contract.sh) return 0 ;;
    esac
    case "$1:$2" in
        ./planning/scripts/plan-map-lib.sh:assoc-array) return 0 ;;
        # The probe that chooses between the GNU and BSD forms must name both.
        ./planning/scripts/plan-document-lib.sh:stat-format) return 0 ;;
        ./planning/scripts/plan-env.sh:stat-format) return 0 ;;
        ./planning/tests/lib-test.sh:stat-format) return 0 ;;
        # Development-only tooling may use python3 (CODE-STYLE.md §1).
        ./benchmark/*:python3-shipped) return 0 ;;
        ./run-tests.sh:python3-shipped) return 0 ;;
        ./planning/tests/*:python3-shipped) return 0 ;;
    esac
    return 1
}

script_list() {
    ( cd "$repo_root" && find . -name '*.sh' -type f \
        -not -path './benchmark/results/*' -not -path './.git/*' -not -path './.plans/*' \
        -not -path './.claude/*' \
        | LC_ALL=C sort )
}

# The generator and this test quote the marker format in their own prose, so they
# are not scanned for markers or constructs.
documents_the_format() {
    case "$1" in
        ./generate-portability.sh|./planning/tests/test-portability-contract.sh) return 0 ;;
    esac
    return 1
}

# 1. Freshness.
if ! "$generator" --check >/dev/null 2>&1; then
    note_fail 'PORTABILITY.md is stale; run ./generate-portability.sh'
fi

# 2 and 3. Marker hygiene.
while IFS= read -r file; do
    documents_the_format "$file" && continue
    while IFS= read -r hit; do
        [ -n "$hit" ] || continue
        line="${hit%%:*}"
        id="${hit#*:}"
        if ! jq -e --arg i "$id" '.rules[]|select(.id==$i)' "$rules" >/dev/null 2>&1; then
            note_fail "$file:$line names unknown rule id '$id' (add it to portability-rules.json)"
        fi
    done < <(awk '/# PORTABILITY\(/ {
                     id = $0
                     sub(/^.*# PORTABILITY\(/, "", id)
                     sub(/\).*$/, "", id)
                     printf "%d:%s\n", FNR, id
                 }' "$repo_root/$file")

    # An untagged marker cannot be indexed into the catalogue.
    while IFS= read -r line; do
        [ -n "$line" ] || continue
        note_fail "$file:$line uses the untagged '# PORTABILITY:' form; use '# PORTABILITY(<rule-id>):'"
    done < <(awk '/# PORTABILITY:/ { print FNR }' "$repo_root/$file")
done < <(script_list)

# 4. No new banned constructs. Comments are stripped first, so prose that names a
# construct (a marker, a docblock) is not mistaken for a use of it.
while IFS= read -r rule_id; do
    detect="$(jq -r --arg i "$rule_id" '.rules[]|select(.id==$i)|.detect' "$rules")"
    [ "$detect" != null ] || continue
    while IFS= read -r file; do
        in_allowlist "$file" "$rule_id" && continue
        # Strip full-line comments and trailing comments, then match. No -q and
        # no -m1: either would close the pipe on the first match and report the
        # writer's SIGPIPE (141) instead of the finding.
        hits="$(sed 's/[[:space:]]*#.*$//' "$repo_root/$file" | grep -nE -- "$detect" || true)"
        if [ -n "$hits" ]; then
            hit="${hits%%$'\n'*}"
            note_fail "$file uses banned construct '$rule_id' at line ${hit%%:*} — see PORTABILITY.md"
        fi
    done < <(script_list)
done < <(jq -r '.rules[].id' "$rules")

[ "$fail" -eq 0 ] || exit 1
printf '%s\n' 'test-portability-contract: PASS'
