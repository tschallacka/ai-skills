#!/usr/bin/env bash
# MODE: PROD
# add-ui-story.sh — add one UI user story row to a plan's ui-user-stories.md and
# create its browser run cache.
#
# The story must name a direct user interaction (click, tap, type, keyboard,
# press, swipe, pinch, drag, select): a story nobody can perform in a browser is
# not evidence. The row starts at "💤 untested" with no evidence; a run fills it.
#
# Usage:
#   add-ui-story.sh [--plan-dir] <plan-directory> --id <US-NN> --persona <text> \
#       --actions <text> --interaction <text> --expected <text> \
#       --work-units <WNN[,WNN...]>
#   add-ui-story.sh [--plan-dir] <plan-directory> <US-NN> <persona-or-precondition> \
#       <browser-actions> <interaction-evidence> <expected-result> <WNN[,WNN...]>
#   add-ui-story.sh --help
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
# plan_hoist_plan_dir call (AR-07): that call rewrites a --plan-dir flag into
# a positional argument, and the compiled binary must receive the caller's
# true original argv, not the already-hoisted form.
aus_bin_pref_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$aus_bin_pref_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present add-ui-story "$aus_bin_pref_script_dir" "$@"
unset aus_bin_pref_script_dir

plan_die "add-ui-story: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
