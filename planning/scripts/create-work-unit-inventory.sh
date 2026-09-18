#!/usr/bin/env bash
# MODE: PROD
# create-work-unit-inventory.sh — seed a plan's work-unit-inventory.md with the
# coverage table, the work-unit table, and the decomposition-review checklist.
#
# create-plan.sh already emits an inventory (without the example rows), so this
# is the repair path for a plan whose inventory was lost: it refuses to overwrite
# an existing one (73). The example rows are placeholders the validator warns
# about until they are replaced by add-coverage.sh / add-work-unit.sh rows.
#
# Usage:
#   create-work-unit-inventory.sh [--plan-dir] <plan-directory>
#   create-work-unit-inventory.sh --help

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed before this script's own
# plan_hoist_plan_dir call below: that call rewrites a --plan-dir flag into a
# positional argument, and the compiled binary must receive the caller's true
# original argv, not the already-hoisted form.
cwui_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$cwui_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present create-work-unit-inventory "$cwui_script_dir" "$@"
unset cwui_script_dir

plan_die "create-work-unit-inventory: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
