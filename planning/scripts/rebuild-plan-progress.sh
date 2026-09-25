#!/usr/bin/env bash
# MODE: PROD
# rebuild-plan-progress.sh — regenerate a plan's progress tracker from its
# goals' own progress files.
#
# Discards the plan-level table and rebuilds it: one row per goal directory that
# holds a goal.md, its Description derived from the goal's Outcome (never a
# literal placeholder), and its status read back from the goal's progress.md
# (`**Progress:** \`100%` means completed, a `⏳ in progress` cell means in
# progress). It resets completion statuses that were set by hand, so callers
# re-apply them with update-step.sh afterwards.
#
# Usage:
#   rebuild-plan-progress.sh [--plan-dir] <plan-directory>
#   rebuild-plan-progress.sh --help
#
# Exit codes: 64 bad invocation, 66 the plan directory, its progress.md, or any
# goal directory is missing.

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
rpp_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$rpp_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present rebuild-plan-progress "$rpp_script_dir" "$@"
unset rpp_script_dir

plan_die "rebuild-plan-progress: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
