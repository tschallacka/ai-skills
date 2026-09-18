#!/usr/bin/env bash
# MODE: PROD
# create-ui-validation.sh — turn a plan into a UI-validating plan: add the
# "## UI validation" section to plan-description.md, flip "UI affected" to yes,
# and create the empty ui-user-stories.md and bugs.md tables.
#
# It refuses to run twice (73): the story and bug tables are authored afterwards
# by add-ui-story.sh and the bug flow, so recreating them would drop that work.
#
# Usage:
#   create-ui-validation.sh [--plan-dir] <plan-directory> <browser-target-or-discovery-method>
#   create-ui-validation.sh --help

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
cuv_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$cuv_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present create-ui-validation "$cuv_script_dir" "$@"
unset cuv_script_dir

plan_die "create-ui-validation: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
