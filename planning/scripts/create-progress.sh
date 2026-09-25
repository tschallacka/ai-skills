#!/usr/bin/env bash
# MODE: PROD
# create-progress.sh — generate a goal's progress.md: one row per implementation
# step, in step-name order, each carrying the step's own Objective text.
#
# Refuses to overwrite an existing tracker (73): rebuilding one is
# update-progress.sh's and rebuild-plan-progress.sh's job, not this script's.
# Testing companions (*-testing.md) are not steps and get no row.
#
# Usage:
#   create-progress.sh <goal-directory> <goal-name>
#   create-progress.sh --help

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
cprg_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$cprg_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present create-progress "$cprg_script_dir" "$@"
unset cprg_script_dir

plan_die "create-progress: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
