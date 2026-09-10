#!/usr/bin/env bash
# MODE: DEV
# test-skill-docs-generation.sh — T87 (SKILL.md/parts are exactly what
# generate-skill-docs.sh produces from skill-source.txt) and T86 (every part
# carries a load-sanity line verify-skill-load.sh actually checks).
#
# Usage: test-skill-docs-generation.sh
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
generator="$repo_dir/planning/scripts/generate-skill-docs.sh"
verifier="$repo_dir/planning/scripts/verify-skill-load.sh"
planning_dir="$repo_dir/planning"

note_fail() { printf 'skill-docs-generation: %s\n' "$1" >&2; t_record "$1"; }

# ── --check passes against the committed tree ───────────────────────────────
if ! "$BASH" "$generator" --check "$planning_dir" >/dev/null; then
    note_fail '--check failed against the committed SKILL.md/parts -- run generate-skill-docs.sh'
fi

# ── determinism: two fresh generations of the same part agree byte for byte ─
work="$(mktemp -d "${TMPDIR:-/tmp}/skill-docs-gen.XXXXXX")"
trap 'rm -rf "$work"' EXIT
copy1="$work/copy1"
copy2="$work/copy2"
mkdir -p "$copy1" "$copy2"
cp "$planning_dir/skill-source.txt" "$copy1/skill-source.txt"
cp "$planning_dir/skill-source.txt" "$copy2/skill-source.txt"
"$BASH" "$generator" "$copy1" >/dev/null
"$BASH" "$generator" "$copy2" >/dev/null
for target in SKILL.md parts/part-1.md parts/part-2.md parts/part-3.md parts/part-4.md; do
    cmp -s "$copy1/$target" "$copy2/$target" \
        || note_fail "two fresh generations of $target differ; generation is not deterministic"
done

# ── --check catches a hand-edited part ───────────────────────────────────────
printf '\nhand edit\n' >> "$copy1/parts/part-2.md"
if "$BASH" "$generator" --check "$copy1" >/dev/null 2>&1; then
    note_fail '--check passed a hand-edited part'
fi

# ── every part carries exactly one load-proof line, and the position is not
#    the same fixed line across parts (a fixed landmark is what T86 exists to
#    avoid: it names the load-proof position as a function of content, not a
#    constant offset from the end) ──────────────────────────────────────────
declare -a offsets_from_end=()
for n in 1 2 3 4; do
    part="$copy2/parts/part-$n.md"
    count="$(grep -c 'SKILL-LOAD-PROOF part=' "$part")"
    [ "$count" -eq 2 ] \
        || note_fail "part-$n should mention SKILL-LOAD-PROOF exactly twice (the instruction line and the real one), found $count"
    proof_line="$(grep -n '^<!-- SKILL-LOAD-PROOF' "$part" | cut -d: -f1)"
    [ -n "$proof_line" ] || note_fail "part-$n has no embedded load-proof line"
    total="$(wc -l < "$part" | tr -d ' ')"
    offsets_from_end+=("$((total - proof_line))")
    # In the last fifth of the file, per T86 ("past the known boundary"),
    # not merely somewhere in it.
    floor=$((total * 4 / 5))
    [ "$proof_line" -ge "$floor" ] \
        || note_fail "part-$n's load-proof line ($proof_line of $total) is not in the last fifth (floor $floor)"
done
first_offset="${offsets_from_end[0]}"
same_offset_everywhere=1
for offset in "${offsets_from_end[@]}"; do
    [ "$offset" = "$first_offset" ] || same_offset_everywhere=0
done
[ "$same_offset_everywhere" -eq 0 ] \
    || note_fail "every part's load-proof sits the same distance from the end ($first_offset lines) -- that is a fixed landmark, not a random position"

# ── verify-skill-load.sh: the actual command T86 requires ───────────────────
real_token="$(grep -oE 'SKILL-LOAD-PROOF part=part-1 token=[0-9a-f]+' "$copy2/parts/part-1.md" \
    | sed 's/.*token=//')"
[ -n "$real_token" ] || note_fail 'could not extract a real token from part-1.md to test against'

if ! "$BASH" "$verifier" --part part-1 --token "$real_token" "$copy2" >/dev/null 2>&1; then
    note_fail 'verify-skill-load.sh refused the real, current token'
fi
if "$BASH" "$verifier" --part part-1 --token deadbeefdeadbeef "$copy2" >/dev/null 2>&1; then
    note_fail 'verify-skill-load.sh accepted a wrong token'
fi
rc=0
"$BASH" "$verifier" --part no-such-part --token "$real_token" "$copy2" >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 66 ] || note_fail "verify-skill-load.sh on a missing part exited $rc, want 66"

# A part regenerated with new content mints a NEW token; the old one, cached
# from before, must stop verifying -- otherwise a stale memorized token would
# silently pass forever, defeating the point of the check.
printf '\n<!-- SKILL_SECTION:START part-1 targets=part-1 -->\nsome new content\n<!-- SKILL_SECTION:END part-1 -->\n' \
    >> "$copy2/skill-source.txt"
"$BASH" "$generator" "$copy2" >/dev/null
if "$BASH" "$verifier" --part part-1 --token "$real_token" "$copy2" >/dev/null 2>&1; then
    note_fail 'a token from before a regeneration still verified after the part changed'
fi

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-skill-docs-generation: PASS'
