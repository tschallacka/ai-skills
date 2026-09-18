#!/usr/bin/env bash
# MODE: PROD
# remove-work-unit.sh — remove a work unit and reconcile every reference to it.
#
# Removes, in one pass so no follow-up call is needed: the inventory row; the id
# from coverage rows (other ids kept; a row is dropped only when it empties);
# the goal's "Owned work units" section (re-derived); the step file and its
# -testing companion; and the goal + plan progress trackers.
#
# Usage:
#   remove-work-unit.sh [--plan-dir] <plan-directory> <WNN> [--confirm-cascade]
#   remove-work-unit.sh --help
#
# Note: the progress trackers are *rebuilt* from the step files, which resets
# completion statuses — re-apply them with update-step.sh afterwards.
#
# Refuses without --confirm-cascade when other work units list this one in
# their Depends-on column; the flag prunes those links (restore them on a
# re-add with `update-work-unit.sh --depends-on`).
#
# Exit codes: 64 bad invocation, unknown id, or a refused cascade; 66 the plan
# directory is missing.
# shellcheck disable=SC2154  # plan_inventory_* are assigned at runtime by the
# sourced plan-inventory-lib row/split helpers

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
rwu_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$rwu_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present remove-work-unit "$rwu_script_dir" "$@"
unset rwu_script_dir

plan_die "remove-work-unit: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
