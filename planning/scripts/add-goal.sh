#!/usr/bin/env bash
# MODE: PROD
# add-goal.sh — create one goal directory (goal.md plus an empty steps/) in a
# plan and refresh the plan-level progress tracker.
#
# The goal's own progress.md is deliberately NOT created here: create-progress.sh
# needs step files to exist, so add-work-unit.sh creates it with the goal's first
# work unit. Fill the emitted placeholders with update-plan-content.sh; the
# Goal-size exception heading is emitted empty on purpose (see below).
#
# Usage:
#   add-goal.sh [--plan-dir] <plan-directory> <goal-name> <title> <outcome>
#   add-goal.sh --help

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed before this script's own
# plan_hoist_plan_dir call (AR-06): that call rewrites a --plan-dir flag into
# a positional argument, and the compiled binary must receive the caller's
# true original argv, not the already-hoisted form.
aag_bin_pref_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$aag_bin_pref_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present add-goal "$aag_bin_pref_script_dir" "$@"
unset aag_bin_pref_script_dir

plan_die "add-goal: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
