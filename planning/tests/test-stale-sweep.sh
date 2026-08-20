#!/usr/bin/env bash
# MODE: PROD
# test-stale-sweep — the --stale wording sweep flags what it claims to flag.
#
# Usage: test-stale-sweep.sh
#
# Report 21 measured this gate against a real 13-failure run and found none of
# the 13 indicated a defect, because three bugs pulled in opposite directions:
#
#   1. the marker exemption was case-sensitive against a lowercase-only
#      alternation, so the capitalised correction note SKILL.md tells authors to
#      write ("An earlier version of this criterion ...") failed the gate;
#   2. buffering was per SECTION, so a marker in any paragraph exempted every
#      paragraph under that heading -- including the unfixed sibling the sweep
#      exists to find, which is its own headline scenario;
#
# This exercises stale_scan_doc directly (the unit that was repaired);
# test-plan-commands.sh covers the validate-plan.sh --stale path end to end.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=planning/scripts/validate-plan-stale-lib.sh
source "$repo_root/planning/scripts/validate-plan-stale-lib.sh"

note_fail() { printf 'stale-sweep: %s\n' "$1" >&2; t_record "$1"; }

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/stale-sweep.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

# write_doc <name> <<'MD' ... MD  -- a fixture document, body on stdin, path in
# $doc. bash 3.2 mis-parses a quote character in a heredoc body inside `$( )`
# global instead of printing the path.
write_doc() {
    doc="$temporary_root/$1.md"
    cat > "$doc"
}

# assert_passes <label> <doc> <phrase>
assert_passes() {
    local label="$1" doc="$2" phrase="$3" hits
    hits="$(stale_scan_doc "$doc" "$phrase")"
    if [ -n "$hits" ]; then
        note_fail "$label: '$phrase' was flagged but must pass -- $hits"
    fi
}

# assert_flags <label> <doc> <phrase> <substring the message must carry>
assert_flags() {
    local label="$1" doc="$2" phrase="$3" expected="$4" hits
    hits="$(stale_scan_doc "$doc" "$phrase")"
    if [ -z "$hits" ]; then
        note_fail "$label: '$phrase' must be flagged and was not"
        return
    fi
    case "$hits" in
        *"$expected"*) ;;
        *) note_fail "$label: message does not name '$expected' -- $hits" ;;
    esac
}

# ── 1. a capitalised history marker is a marker ───────────────────────────────
write_doc capitalised <<'MD'
## Testing requirement

An earlier version of this blurb named only the first two, while the unit's own
instructions, acceptance and companion require all three.
MD
assert_passes 'capitalised marker' "$doc" 'all three'

# ── 4. the lowercase control: the exemption itself still works ────────────────
write_doc lowercase <<'MD'
## Testing requirement

an earlier version of this blurb named only the first two, while the unit's own
instructions, acceptance and companion require all three.
MD
assert_passes 'lowercase marker' "$doc" 'all three'

# ── 2. a marked paragraph must not shield an unmarked sibling ─────────────────
write_doc half_landed <<'MD'
## Instructions

The wording changed here; an earlier version said something else entirely.

The renderer emits all four states unconditionally.
MD
assert_flags 'half-landed fix' "$doc" 'all four' \
    'The renderer emits all four states unconditionally.'
assert_flags 'half-landed fix names the paragraph' "$doc" 'all four' '[paragraph 2]'

# ── the gate is not disabled: an unmarked count still fails ───────────────────
write_doc bare_count <<'MD'
## Acceptance

The renderer emits all four states unconditionally.
MD
assert_flags 'bare count' "$doc" 'all four' '[paragraph 1]'

write_doc prose_commas <<'MD'
## Acceptance

The grader walks all four states, which the renderer emits, and then compares
the totals, the labels, and the ordering against the recorded baseline.
MD
assert_flags 'prose commas' "$doc" 'all four' 'The grader walks all four states'

write_doc class_b <<'MD'
## Acceptance

The generated PDF is byte-identical to before: same bytes, same length, same
checksum, same trailer.
MD
assert_flags 'no-numeral phrase' "$doc" 'byte-identical' '[paragraph 1]'

# Text above the first heading is out of scope, as it always was.
write_doc preamble <<'MD'
The renderer emits all four states unconditionally.

## Acceptance

Nothing to see here.
MD
assert_passes 'preamble before any heading' "$doc" 'all four'

# ---- severity: a count blocks, an identical-output wording only warns --------
# Class B is 50% precise as a gate (an identical PDF is a defect,
# `byte-for-byte` in a JSON file is correct), so it points at the Artifact
# comparisons table, which checks the same thing exactly.
scripts_dir="$repo_root/planning/scripts"
sev_plan="$temporary_root/severity-plan"
cp -R "$repo_root/benchmark/planning/tests/fixtures/review-lifecycle-plan" "$sev_plan"
sev_doc="$(find "$sev_plan" -type f -name '*-testing.md' | LC_ALL=C sort | head -1)"
sev_pristine="$temporary_root/severity.pristine"
cp "$sev_doc" "$sev_pristine"

gate_output() { # <paragraph text> [--stale argument]
    cp "$sev_pristine" "$sev_doc"
    printf '\n## Automated tests\n\n%s\n' "$1" >> "$sev_doc"
    { "$scripts_dir/validate-plan.sh" --stale "${2:-default}" "$sev_plan" 2>&1 || true; }
}

out="$(gate_output 'The adapter output is byte-identical to the recorded evidence.')"
case "$out" in
    *"WARN: wording 'byte-identical'"*) ;;
    *) note_fail 'an identical-output wording did not produce a warning' ;;
esac
case "$out" in
    *"FAIL: stale phrase 'byte-identical'"*) note_fail 'an identical-output wording still fails the gate' ;;
esac
case "$out" in
    *'Artifact comparisons'*) ;;
    *) note_fail 'the warning does not name the table that checks this exactly' ;;
esac

# A count warns rather than blocking: on the real plans it found 0 defects in 24
# hits, every one an accurate count of a fixed set.
out="$(gate_output 'The grader walks all four states in the recorded order.')"
case "$out" in
    *"WARN: count 'all four'"*) ;;
    *) note_fail 'a bare count did not produce a warning' ;;
esac
case "$out" in
    *"FAIL: stale phrase"*|*"FAIL: count "*) note_fail 'the stale sweep still fails the gate' ;;
esac
case "$out" in
    *'enumerate the items'*) ;;
    *) note_fail 'the count warning does not say what to write instead' ;;
esac

# A caller-supplied list has no known class, so nothing in it is downgraded.
printf 'byte-identical\n' > "$temporary_root/supplied-phrases"
out="$(gate_output 'The adapter output is byte-identical to the evidence.' "$temporary_root/supplied-phrases")"
case "$out" in
    *"WARN: count 'byte-identical'"*) ;;
    *) note_fail 'a caller-supplied phrase produced no finding' ;;
esac
# A supplied list replaces the bundled one, so the bundled comparison phrases
# must not also fire -- which would show up as a second, differently-worded
# warning for the same phrase.
case "$out" in
    *"WARN: wording 'byte-identical'"*)
        note_fail 'the bundled comparison list ran for a caller-supplied phrase file' ;;
esac
cp "$sev_pristine" "$sev_doc"

echo
echo "stale-sweep: $([ "$(t_failures)" -eq 0 ] && echo PASS || echo FAIL)"
[ "$(t_failures)" -eq 0 ]
