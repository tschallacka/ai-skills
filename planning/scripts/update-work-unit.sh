#!/usr/bin/env bash
# MODE: PROD
# update-work-unit.sh — amend a work-unit inventory row and its matching step
# file in place.
#
# Usage:
#   update-work-unit.sh [--plan-dir] <plan-directory> <WNN> [<new-primary-scope>] [<new-file>]
#                       [--scope <text>] [--file <path>] [--type <type>]
#                       [--depends-on <WNN[,WNN...]|—>] [--description <text>]
#   update-work-unit.sh --help
#
# The inventory row columns are: | ID | Type | File | Primary symbol or file
# scope | Subscope | Intended change | Depends on | Goal | Step |. The third
# positional updates *Primary scope* (column 5); the optional fourth updates
# *File* (column 4). An empty positional leaves its column unchanged, so
# `"" "<path>"` updates File without touching scope. --scope/--file are the
# flag forms (equivalent to the positionals, for callers who prefer flags);
# flags update the remaining columns; nothing else changes, so coverage rows,
# the goal Owned work units section, and progress trackers are untouched —
# changing a dependency must never go through remove + re-add (that would drop
# the unit from its coverage rows and require manual repair).
#
# Exit codes: 64 bad invocation or malformed value, 66 plan directory missing.
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
uwu_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$uwu_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present update-work-unit "$uwu_script_dir" "$@"
unset uwu_script_dir

plan_die "update-work-unit: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
