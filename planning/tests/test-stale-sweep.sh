#!/usr/bin/env bash
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
#   3. a count already discharged by an enumeration in the same paragraph -- the
#      drift-proof form the rule asks authors to produce -- was still flagged.
#
# This exercises stale_scan_doc directly (the unit that was repaired);
# test-plan-commands.sh covers the validate-plan.sh --stale path end to end.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=planning/scripts/validate-plan-stale-lib.sh
source "$repo_root/planning/scripts/validate-plan-stale-lib.sh"

fail=0
note_fail() { printf 'stale-sweep: %s\n' "$1" >&2; fail=1; }

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

# ── 3. a count discharged by an enumeration is the target form ────────────────
write_doc enumerated_colon <<'MD'
## Objective

Unit-test getInvoiceGroups across all six cases its instructions enumerate: a
full invoice, a partial invoice where invoiced quantity is below ordered
quantity, an invoice with no split, malformed split data, the fractional
two-employees-three-units case, and recorded attribution versus the derived
fallback.
MD
assert_passes 'enumeration after a colon' "$doc" 'all six'

write_doc enumerated_ids <<'MD'
## Approach

Place the five overrides in this module so all five are new files copied from
their live source and then grouped, the two containers (W10, W11), the
re-pointed quantity cell (W23), and the two item renderers carrying
bucket-scoped money (W26 on the order screen, W24 on the invoice screen).
MD
assert_passes 'enumeration by work-unit id' "$doc" 'all five'

# The em-dash introducer, without work-unit ids to fall back on.
write_doc enumerated_dash <<'MD'
## Approach

The renderer covers all three states — pending, shipped, and cancelled.
MD
assert_passes 'enumeration after an em dash' "$doc" 'all three'

# ── the gate is not disabled: an unmarked, unenumerated count still fails ─────
write_doc bare_count <<'MD'
## Acceptance

The renderer emits all four states unconditionally.
MD
assert_flags 'bare count' "$doc" 'all four' '[paragraph 1]'

# Prose commas are not an enumeration: no introducer, so no exemption.
write_doc prose_commas <<'MD'
## Acceptance

The grader walks all four states, which the renderer emits, and then compares
the totals, the labels, and the ordering against the recorded baseline.
MD
assert_flags 'prose commas' "$doc" 'all four' 'The grader walks all four states'

# A phrase with no numeral gets no enumeration exemption at all.
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

echo
echo "stale-sweep: $([ "$fail" -eq 0 ] && echo PASS || echo FAIL)"
[ "$fail" -eq 0 ]
