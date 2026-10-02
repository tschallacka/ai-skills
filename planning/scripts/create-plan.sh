#!/usr/bin/env bash
# MODE: PROD
# create-plan.sh — create a plan directory with its plan-description.md,
# work-unit-inventory.md, commands.json, env files, and initial git commit.
#
# Where the plan lands depends on the argument: a path (containing "/") is used
# verbatim, a bare name resolves the plans root via plan-root.sh, prompting on
# first use in a project. Which repository owns the plan's history is decided
# further down, next to the rules it follows.
#
# Usage:
#   create-plan.sh <plan-name|plan-directory> <title>
#   create-plan.sh --help

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
cp_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$cp_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present create-plan "$cp_script_dir" "$@"
unset cp_script_dir

plan_die "create-plan: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
