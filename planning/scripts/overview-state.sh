#!/usr/bin/env bash
# MODE: PROD
# overview-state.sh - emit the complete reviewing state of one plan as a
# single JSON document on stdout. This document is the one source that both
# delivery modes (file artifact and served page) render from, so they cannot
# disagree about what the plan says.
#
# Usage:
#   overview-state.sh [--plan-dir] <plan-directory>
#   overview-state.sh --help
#
# Exit codes: 64 bad invocation, 66 missing plan directory.

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed before this script's own plan-dir
# hoist call below: that call rewrites a --plan-dir flag into a positional
# argument, and the compiled binary must receive the caller's true original
# argv, not the already-hoisted form. The compiled binary itself was extended
# to parse --plan-dir/--plan-dir= directly (src/plan-overview/src/bin/overview-state.rs).
os_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$os_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present overview-state "$os_script_dir" "$@"
unset os_script_dir

plan_die "overview-state: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
