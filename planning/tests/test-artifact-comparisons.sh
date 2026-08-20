#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# test-artifact-comparisons.sh — a declared artifact comparison must be one the
# target can actually produce.
#
# Why this exists: the --stale sweep looks for `byte-identical` in prose and
# cannot tell a PDF from a JSON file. Measured against a labelled corpus it was
# right about half the time -- it correctly failed an identical-PDF criterion and
# wrongly failed `byte-for-byte in oracle-terminal-evidence.json`. The
# discriminating fact is the artifact, so declaring it as data makes the check
# exact instead of a keyword guess.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/artifact-comparisons.XXXXXX")"
trap 'rm -rf "$work"' EXIT

plan="$work/plan"
cp -R "$repo_root/benchmark/planning/tests/fixtures/review-lifecycle-plan" "$plan"
companion="$(find "$plan" -type f -name '*-testing.md' | LC_ALL=C sort | head -1)"
[ -n "$companion" ] || t_fail 'the fixture has no testing companion to write into'
pristine="$work/companion.pristine"
cp "$companion" "$pristine"

# <rows> -> the gate's findings for a companion declaring exactly those rows
findings_for() {
    cp "$pristine" "$companion"
    {
        printf '\n## Artifact comparisons\n\n'
        printf '| Artifact | Comparison |\n|---|---|\n'
        printf '%s\n' "$1"
    } >> "$companion"
    # validate-plan.sh exits 1 when it finds a finding, which is what we are
    # measuring, so its status must not abort the test under pipefail.
    { "$scripts_dir/validate-plan.sh" "$plan" 2>&1 || true; } |
        { grep -E 'cannot be compared|is not in artifact-comparisons' || true; }
}

expect_clean() { # <label> <rows>
    local out
    out="$(findings_for "$2")"
    [ -z "$out" ] || t_fail "$1: reported a finding for a legal comparison — $out"
}

expect_refused() { # <label> <rows> <needle>
    local out
    out="$(findings_for "$2")"
    [ -n "$out" ] || t_fail "$1: a comparison the target cannot produce was accepted"
    t_assert_contains "$1" "$3" "$out"
}

# A deterministic target may be compared byte for byte. This is the case the
# prose sweep failed, and the reason this check exists.
expect_clean 'deterministic json, exact'  '| `oracle-terminal-evidence.json` | exact |'
expect_clean 'plain text, exact'          '| `notes.txt` | exact |'
expect_clean 'pdf by text layer'          '| `pub/media/invoice.pdf` | text-layer |'
expect_clean 'image, perceptual'          '| `chart.png` | perceptual |'
expect_clean 'no extension at all'        '| `generated-output` | exact |'

# A target that embeds a timestamp, encoder version or per-entry metadata cannot
# be reproduced byte for byte, so demanding it fails correct work.
expect_refused 'pdf, exact'   '| `pub/media/invoice.pdf` | exact |' 'creation timestamp'
expect_refused 'image, exact' '| `chart.png` | exact |'             'encoder version'
expect_refused 'docx, exact'  '| `report.docx` | exact |'           'timestamps'
expect_refused 'uppercase extension is still a pdf' '| `INVOICE.PDF` | exact |' 'creation timestamp'

# An unregistered comparison is refused too, or the vocabulary means nothing.
expect_refused 'comparison not in the registry' '| `pub/media/invoice.pdf` | eyeballed |' \
    'not in artifact-comparisons.json'

# A companion with no such section is not the pass's business.
cp "$pristine" "$companion"
out="$({ "$scripts_dir/validate-plan.sh" "$plan" 2>&1 || true; } | { grep -E 'cannot be compared|artifact-comparisons' || true; })"
[ -z "$out" ] || t_fail "a companion with no comparisons section produced a finding — $out"

# The section is a table, so per CODE-CONTRACTS.md contract 1 it must NOT be a
# section-form target: `-ss ... artifact-comparisons` would rewrite it as
# paragraphs and discard every row. It is authored with -tp, and the guard in
# plan_replace_section refuses the destructive form.
kind_list="$("$BASH" -c '
    plan_error_count=0
    source "$1/plan-map-lib.sh"; source "$1/plan-inventory-lib.sh"
    source "$1/plan-document-lib.sh"
    sed -n "s/^        testing) valid=\"\([^\"]*\)\".*/\1/p" "$1/plan-document-lib.sh"
' _ "$scripts_dir" 2>/dev/null || true)"
case "$kind_list" in
    *artifact-comparisons*)
        t_fail 'artifact-comparisons is a table but is listed as a mutable narrative section' ;;
esac

t_end
