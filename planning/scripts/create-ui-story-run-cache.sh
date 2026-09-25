#!/usr/bin/env bash
# MODE: PROD
# create-ui-story-run-cache.sh — create the empty browser run cache for one UI
# story: its starting state, one buffered interaction, its readiness wait, and an
# untested run result.
#
# add-ui-story.sh calls this, so the cache exists from the moment the story does.
# Every field is the literal "not yet configured" until
# configure-ui-story-cache.sh writes the real sequence; run that next.
#
# Usage:
#   create-ui-story-run-cache.sh [--plan-dir] <plan-directory> <US-NN>
#   create-ui-story-run-cache.sh --help

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
cusrc_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$cusrc_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present create-ui-story-run-cache "$cusrc_script_dir" "$@"
unset cusrc_script_dir

plan_die "create-ui-story-run-cache: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
