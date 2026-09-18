#!/usr/bin/env bash
# MODE: PROD
# plan-content.sh — read-only queries over one plan's documents.
#
# Five subcommands, all non-mutating: `get` prints one document, `summary`
# renders the work-unit inventory, `blast-radius` walks the depends-on graph
# from a unit/goal/step, `find` does a literal single-hit search that exits 1
# unless exactly one line matches, and `diff` maps changed lines since a git ref
# back to the enclosing "§ N.N" paragraph labels.
#
# Usage:
#   The plan directory may be given positionally or as --plan-dir <path>.
#   plan-content.sh get|summary|blast-radius|find|diff [--plan-dir] <plan-directory> [...]
#   plan-content.sh --help
#
# Exit codes: 1 zero or multiple `find` matches, 64 bad invocation, 66 missing
# document.
# shellcheck disable=SC2154  # plan_inventory_* are assigned at runtime by the
# sourced plan-inventory-lib row/split helpers

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. This script's own plan_hoist_plan_dir call
# runs well after both of the anchor lines above, not between them, so the
# standard after-both-anchors placement is already correct here too.
pc_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$pc_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present plan-content "$pc_script_dir" "$@"
unset pc_script_dir

plan_die "plan-content: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
