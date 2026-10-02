#!/usr/bin/env bash
# MODE: PROD
# cleanup-plans.sh — list completed plans and remove selected ones.
#
# Usage:
#   cleanup-plans.sh [-l|--list] [<plan-name> ...] [-y|--yes]
#   cleanup-plans.sh --help
#
# With no plan names: lists every plan under the resolved plans root and marks
# the completed ones (plan-level progress.md at 100%). With plan names: removes
# each named plan after confirmation, using remove-plan.sh (which clears the
# plans-root git history when the last plan is removed). --yes skips the
# confirmation prompt for non-interactive runs.
#
# Plan names are matched against plan directory names (kebab-case). An unknown
# name is an error, not a silent skip, so a typo cannot remove the wrong plan.
#
# Exit codes: 1 = the confirmation prompt was declined; 66 = the plans root or a
# named plan does not exist.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
cup_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$cup_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present cleanup-plans "$cup_script_dir" "$@"
unset cup_script_dir

plan_die "cleanup-plans: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
