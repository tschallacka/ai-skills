#!/usr/bin/env bash
# MODE: PROD
# plan-context-wrapper.sh — source a per-worker variables file, then exec
# plan-context.sh with the remaining arguments.
#
# The wrapper exists so a worker can supply benign context defaults (RUN_ID,
# REVISION, NEXT_ACTION) without every caller having to export them. It writes
# nothing and holds no shared state; the caller owns the short-lived file.
#
# Usage:
#   plan-context-wrapper.sh <variables-file> <plan-context.sh arguments...>
#   plan-context-wrapper.sh --help
#
# Exit codes: 64 bad invocation, 66 variables file missing.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
pcw_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$pcw_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present plan-context-wrapper "$pcw_script_dir" "$@"
unset pcw_script_dir

plan_die "plan-context-wrapper: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
