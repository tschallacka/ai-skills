#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# validate-plan-comparisons-lib.sh — an acceptance criterion may not demand an
# identical output the target cannot produce.
#
# The `--stale` sweep looks for this in prose (`byte-identical`, `pixel-identical`)
# and cannot tell a PDF from a JSON file, so it fails correct work about half the
# time. A step's testing companion may instead declare its comparisons as a
# table, which makes the check exact:
#
#   ## Artifact comparisons
#
#   | Artifact | Comparison |
#   |---|---|
#   | `pub/media/invoice.pdf` | text-layer |
#   | `oracle-terminal-evidence.json` | exact |
#
# Author it with `update-plan-content.sh -tp <plan> step:<goal>/<step>-testing
# <N.N> 'Artifact,Comparison' '<csv>'`, never by hand.
#
# Sourced by validate-plan.sh; never executed. Requires
# validate-plan-common-lib.sh. Reads `skill_root` for the registry path.

# shellcheck disable=SC2154
# The entry script and its sibling libs publish these globals; shellcheck lints
# each file alone and cannot see the assignments.
set -euo pipefail

artifact_comparison_registry="$skill_root/artifact-comparisons.json"

# The registry is the source of truth for both halves of the rule: which
# comparisons exist at all, and which artifacts cannot be reproduced byte for
# byte. Absent registry is a hard stop, not a silent pass.
plan_validate_artifact_comparisons() {
    [ -f "$artifact_comparison_registry" ] || {
        fail "Artifact comparison registry not found: $artifact_comparison_registry"
        return 0
    }
    local legal nondet companion section artifact comparison extension reason line
    legal=" $(jq -r '.comparisons | keys[]' "$artifact_comparison_registry" 2>/dev/null | tr '\n' ' ')"
    [ "$legal" != " " ] || { fail "Artifact comparison registry lists no comparisons"; return 0; }

    while IFS= read -r companion; do
        [ -n "$companion" ] || continue
        # Only the rows under the comparisons heading; a table elsewhere in the
        # companion is none of this pass's business.
        section="$(awk '
            /^## Artifact comparisons$/ { inside = 1; next }
            inside && /^## / { exit }
            inside && /^\|/ { print }
        ' "$companion")"
        [ -n "$section" ] || continue
        while IFS= read -r line; do
            [ -n "$line" ] || continue
            case "$line" in
                *'---'*) continue ;;
            esac
            # trim from validate-plan-common-lib.sh already strips surrounding
            # whitespace and backticks, so a row needs no awk of its own.
            IFS='|' read -r _ artifact comparison _ <<<"$line"
            artifact="$(trim "$artifact")"
            comparison="$(trim "$comparison")"
            [ -n "$artifact" ] && [ -n "$comparison" ] || continue
            case "$artifact" in
                Artifact) continue ;;
            esac
            case "$legal" in
                *" $comparison "*) ;;
                *) fail "$companion: comparison '$comparison' for $artifact is not in artifact-comparisons.json (legal:${legal% })"
                   continue ;;
            esac
            [ "$comparison" = exact ] || continue
            extension="${artifact##*.}"
            [ "$extension" != "$artifact" ] || continue
            extension="$(printf '%s' "$extension" | tr '[:upper:]' '[:lower:]')"
            reason="$(jq -r --arg e "$extension" '.nondeterministic_extensions[$e] // empty' \
                "$artifact_comparison_registry" 2>/dev/null)"
            [ -n "$reason" ] || continue
            fail "$companion: $artifact cannot be compared 'exact' -- $reason. Use one of the non-exact comparisons in artifact-comparisons.json and say what tolerance the proof allows."
        done <<COMPARISON_ROWS
$section
COMPARISON_ROWS
    done <<COMPANIONS
$(find "$plan_dir" -type f -name '*-testing.md' -not -path '*/context/*' | LC_ALL=C sort)
COMPANIONS
}
