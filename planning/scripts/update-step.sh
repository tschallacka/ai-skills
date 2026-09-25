#!/usr/bin/env bash
# MODE: PROD
# update-step.sh — set one step's completion status in its goal's tracker.
#
# Rewrites the step's row in <goal-directory>/progress.md (canonical 4 data
# columns, so awk -F'|' matches the step name in $3 and replaces the trailing
# status cell), then re-derives the goal's bar by invoking update-progress.sh.
# It refuses when the step row is absent or appears more than once, because
# guessing which row to edit would silently corrupt the tracker.
#
# The child's progress line goes to stderr: stdout carries exactly this
# script's own one-line result (CODE-STYLE §10).
#
# Usage:
#   update-step.sh <goal-directory> <step-name> <incomplete|in-progress|completed>
#   update-step.sh --help
#
# Exit codes: 1 the step row is not present exactly once, 64 bad invocation,
# 66 the goal has no progress.md.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
ust_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$ust_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present update-step "$ust_script_dir" "$@"
unset ust_script_dir

plan_die "update-step: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
