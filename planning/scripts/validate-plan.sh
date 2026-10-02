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
#   4b. coherence     — B110: intra-document self-coherence, exact, FAILs
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

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism and why PLANNING_SKILL_ROOT is exported
# unconditionally.
vp_bin_pref_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$vp_bin_pref_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present validate-plan "$vp_bin_pref_script_dir" "$@"
unset vp_bin_pref_script_dir

plan_die "validate-plan: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
