#!/usr/bin/env bash
# MODE: PROD
# configure-ui-story-cache.sh — fill a UI story's browser run cache with the one
# buffered interaction the story will actually perform, plus its readiness wait.
#
# This rewrites the whole cache file from the arguments (it is a generated
# document, so hand edits are not preserved) and resets the run result to
# untested: a configured sequence has by definition not been run yet.
#
# Usage:
#   configure-ui-story-cache.sh [--plan-dir] <plan-directory> --id <US-NN> \
#       --starting-state <text> --input <direct UI input> --target <text> \
#       --readiness <text> --max-wait <text>
#   configure-ui-story-cache.sh [--plan-dir] <plan-directory> <US-NN> <starting-state> \
#       <direct-ui-input> <target-or-value> <readiness-signal> <maximum-wait>
#   configure-ui-story-cache.sh --help
#
# The second form is the deprecated positional spelling, kept working for
# existing callers.

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
cusc_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$cusc_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present configure-ui-story-cache "$cusc_script_dir" "$@"
unset cusc_script_dir

plan_die "configure-ui-story-cache: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
