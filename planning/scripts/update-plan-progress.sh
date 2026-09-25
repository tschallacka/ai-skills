#!/usr/bin/env bash
# MODE: PROD
# update-plan-progress.sh — set one goal's status row in the plan-level tracker
# and recompute the plan's overall bar.
#
# Rewrites the goal's row in [--plan-dir] <plan-directory>/progress.md (canonical 3 data
# columns: Goalname | Description | Completion status, so awk -F'|' reads the
# status from $4), then re-derives `**Overall progress:**` from every row. It
# refuses when the goal row is absent or appears more than once, because
# guessing which row to edit would silently corrupt the tracker.
#
# Usage:
#   update-plan-progress.sh [--plan-dir] <plan-directory> <goal-name> <incomplete|in-progress|completed>
#   update-plan-progress.sh --help
#
# Exit codes: 1 the goal row is not present exactly once, 64 bad invocation,
# 66 the plan has no progress.md.

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
upp_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$upp_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present update-plan-progress "$upp_script_dir" "$@"
unset upp_script_dir

plan_die "update-plan-progress: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
