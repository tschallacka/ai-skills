#!/usr/bin/env bash
# MODE: PROD
# validate-plan.sh — gate a plan directory against the planning contract and
# report every finding, not just the first.
#
# This file owns argument parsing and the ORDER of the passes; each pass lives
# in a sourced validate-plan-*-lib.sh sibling. The order is load-bearing:
#   0. obsolescence   — an OBSOLETE marker refuses the plan outright (else stop)
#   1. existence      — the two required documents are present (else stop)
#   2. plan docs      — headings, UI verdict, review verdict, hand-edit damage
#   3. placeholders   — registered template tokens (WARN, or FAIL when generated)
#   4. stale          — --stale phrase sweep, advisory: WARN only, never gates
#   5. inventory      — parse the work-unit table and build the data model
#   6. dependencies   — cycles and unknown edges
#   7. proof coverage — KNOWN DEAD, see validate-plan-inventory-lib.sh
#   8. UI             — user stories, run caches, bugs.md
#   9. goals/steps    — goal.md and step files agree with the inventory
#  10. still serves   — a state-changing goal verifies the running application
#  11. commands       — unregistered command literals
#  12. completion     — --complete: progress trackers agree
#  13. propagation    — the surfaces of a work unit agree
#  14. comparisons    — a declared artifact comparison is achievable
#
# Usage:
#   validate-plan.sh [--complete] [--propagation|--no-propagation]
#                    [--stale <file-of-phrases>|default] [--repo-root DIR]
#                    <plan-directory>
#   validate-plan.sh --help
#   The plan directory may be given positionally or as --plan-dir <path>.
#
# Exit codes: 0 clean, 1 findings, 64 bad invocation, 65 the plan is marked
# obsolete and must not be used, 66 plan directory absent.

set -euo pipefail
export LC_ALL=C

case "${1:-}" in
    -h|--help)
        echo "Usage: $(basename "$0") [--complete] [--propagation|--no-propagation] [--stale <file-of-phrases>|default] [--repo-root DIR] [--plan-dir] <plan-directory>" >&2
        exit 0
        ;;
esac

complete_mode=false
propagation_mode=true
stale_file=""
stale_only=false
plan_dir_only=false
stale_requested=false
repo_root=""
filtered_args=()
for arg in "$@"; do
    case "$arg" in
        --complete) complete_mode=true ;;
        --propagation) propagation_mode=true ;;
        --no-propagation) propagation_mode=false ;;
        --stale)
            stale_only=true
            stale_requested=true
            ;;
        --plan-dir) plan_dir_only=true ;;
        --plan-dir=*) filtered_args+=("${arg#--plan-dir=}") ;;
        --stale=*)
            stale_file="${arg#--stale=}"
            stale_requested=true
            ;;
        --repo-root) repo_root_pending=true ;;
        --repo-root=*) repo_root="${arg#--repo-root=}" ;;
        --)
            filtered_args+=("$arg")
            ;;
        *)
            if [ "${repo_root_pending:-false}" = true ]; then
                repo_root="$arg"
                repo_root_pending=false
            elif [ "$stale_only" = true ]; then
                stale_file="$arg"
                stale_only=false
            elif [ "$plan_dir_only" = true ]; then
                filtered_args+=("$arg")
                plan_dir_only=false
            else
                filtered_args+=("$arg")
            fi
            ;;
    esac
done
# PORTABILITY(empty-array-setu)
set -- ${filtered_args[@]+"${filtered_args[@]}"}

if [ "$#" -eq 1 ]; then
    plan_dir="$1"
elif [ "$#" -eq 2 ] && [ "$1" = '--complete' ]; then
    complete_mode=true
    plan_dir="$2"
else
    echo "Usage: $(basename "$0") [--complete] [--propagation] [--stale <file-of-phrases>|default] [--repo-root DIR] [--plan-dir] <plan-directory>" >&2
    exit 64
fi
[ "${repo_root_pending:-false}" = false ] || { echo "Usage: $(basename "$0") --repo-root requires a directory" >&2; exit 64; }
[ -z "$repo_root" ] || [ -d "$repo_root" ] || { echo "validate-plan.sh: repository root not found: $repo_root" >&2; exit 66; }

inventory="$plan_dir/work-unit-inventory.md"
errors=0

# Resolve the skill root ONCE here for the registries the libraries read:
# ${BASH_SOURCE[0]} inside a sourced library names the library, not this
# script, so a lib deriving its own path breaks the moment it moves.
# ---- quoted: registries read by the libraries ----
# placeholders.json
# state-change-registry.json
# never-executable-extensions.json
# goal-tables.json
# ---- end quoted ----
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
skill_root="$(cd "$script_dir/.." && pwd)"

# One accumulating cleanup for the whole process. A library that needs a temp
# file appends to this list; nothing installs its own EXIT trap, because
# `trap - EXIT` to release one would discard this handler too (CODE-STYLE.md §8).
cleanup_files=()
plan_validate_cleanup() {
    [ "${#cleanup_files[@]}" -eq 0 ] || rm -f ${cleanup_files[@]+"${cleanup_files[@]}"}
}
trap plan_validate_cleanup EXIT

# Refuse without jq rather than degrade: every jq call in the pass libraries is
# `2>/dev/null`, so the placeholder registry, the serve check and part of
# command-literal detection would silently stop firing and still exit 0.
if ! command -v jq >/dev/null 2>&1; then
    printf '%s: jq is required (it reads placeholders.json, state-change-registry.json, goal-tables.json and commands.json); install jq and re-run\n' \
        "${0##*/}" >&2
    exit 69
fi

# Associative arrays for bash 3.2: `declare -A` aborts on stock macOS, where
# this gate has to run.
# shellcheck source=planning/scripts/plan-map-lib.sh
source "$script_dir/plan-map-lib.sh"
# For plan_duplicate_step_numbers. The core library carries no load-time
# initialisation and no trap, so sourcing it here adds definitions only.
source "$script_dir/plan-core-lib.sh"
source "$script_dir/validate-plan-common-lib.sh"
source "$script_dir/validate-plan-docs-lib.sh"
source "$script_dir/validate-plan-placeholders-lib.sh"
source "$script_dir/validate-plan-stale-lib.sh"
source "$script_dir/validate-plan-inventory-lib.sh"
source "$script_dir/validate-plan-ui-lib.sh"
source "$script_dir/validate-plan-goals-lib.sh"
source "$script_dir/validate-plan-serve-lib.sh"
source "$script_dir/validate-plan-commands-lib.sh"
source "$script_dir/validate-plan-propagation-lib.sh"
source "$script_dir/validate-plan-comparisons-lib.sh"

plan_validate_obsolete || exit "$?"
plan_validate_existence || exit "$?"
plan_validate_plan_docs
plan_validate_step_numbers "$plan_dir"
plan_validate_placeholders
plan_validate_stale
plan_validate_inventory
plan_validate_dependency_graph
plan_validate_inventory_target_paths
plan_validate_proof_coverage
plan_validate_ui
plan_validate_goals
plan_validate_step_files
plan_validate_step_naming
plan_validate_still_serves
plan_validate_commands
plan_validate_completion
plan_validate_artifact_comparisons
if [ "$propagation_mode" = true ]; then
    plan_validate_propagation_symbols
    plan_validate_propagation_reach
    plan_validate_propagation_companion
    plan_validate_propagation_leaves
    plan_validate_propagation_roster
    plan_validate_propagation_freshness
fi

if [ "$errors" -gt 0 ]; then
    printf 'Plan validation failed with %d error(s).\n' "$errors" >&2
    exit 1
fi

# "passed" is reserved for a plan with nothing left to fill. A registered
# placeholder is a WARN, not an error, so this still exits 0 -- but the summary
# said "passed" over 138 placeholders in a real run, including
# `<direct action on this one target>` sitting in every step's Instructions, and
# that is the line a reader takes as the readiness verdict.
# shellcheck disable=SC2154  # unit_ids is published by the inventory lib;
# placeholder_warnings by the placeholder lib.
if [ "${placeholder_warnings:-0}" -gt 0 ]; then
    printf 'Plan validation incomplete: %d work units across %d goals, %d placeholder(s) still to fill in %d document(s).\n' \
        "${#unit_ids[@]}" "$(plan_map_count goal_units)" "$placeholder_warnings" \
        "$(printf '%s' "$placeholder_warning_docs" | wc -w | tr -d ' ')"
    printf 'Structure is sound; fill the placeholders above before presenting the plan, or run with --complete to have them reported as errors.\n'
else
    printf 'Plan validation passed: %d work units across %d goals.\n' \
        "${#unit_ids[@]}" "$(plan_map_count goal_units)"
fi
