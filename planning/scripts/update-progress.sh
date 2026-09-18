#!/usr/bin/env bash
# MODE: PROD
# update-progress.sh — recompute a goal's progress bar from its own step rows.
#
# Reads the goal's progress.md step table (canonical 4 data columns: Goalname |
# Stepname | Description | Completion status, so awk -F'|' sees the status in
# $5) and rewrites the single `**Progress:**` line with the percentage, a
# 20-cell bar, and a status icon. It never touches step rows — update-step.sh
# owns those, and calls this script afterwards.
#
# Usage:
#   update-progress.sh <goal-directory>
#   update-progress.sh --help
#
# Exit codes: 64 bad invocation, 66 the goal has no progress.md.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
up_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$up_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present update-progress "$up_script_dir" "$@"
unset up_script_dir

plan_die "update-progress: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
