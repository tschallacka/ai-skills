#!/usr/bin/env bash
# MODE: PROD
# update-ui-story.sh — correct the narrative columns of one UI story row, or
# record the result of running it.
#
# add-ui-story.sh appends and add-ui-story-links.sh rewrites the related work
# units, but nothing could correct a story's own text, so a story that turned out
# to contradict the plan had to be left wrong or edited by hand. Editing plan
# files by hand is what the helpers exist to prevent.
#
# The interaction rule is re-checked against the resulting row, not the arguments:
# correcting only the actions column could otherwise leave a row whose remaining
# text names no interaction at all.
#
# --status and --evidence record a run. They write the row's Status/Evidence
# columns AND the matching `- Status:`/`- Evidence:` lines in the story's own
# browser run cache (ui-story-runs/US-NN.md), which is the second copy
# validate-plan-ui-lib.sh reads at --complete (B114: before this, nothing could
# write either, so a story that was actually run stayed "untested" forever).
# Status must be one of the vocabulary in
# planning/references/ui-user-story-validation.md; passed, bug found and
# excluded all require --evidence, and excluded requires the user's approval
# recorded in it.
#
# Usage:
#   update-ui-story.sh [--plan-dir] <plan-directory> <US-NN>
#       [--persona <text>] [--actions <text>] [--interaction <text>]
#       [--expected <text>] [--status <value>] [--evidence <text>]
#   update-ui-story.sh --help
#
# Exit codes: 64 bad invocation, 66 the plan file or the story is missing.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. This script's own plan_hoist_plan_dir call
# runs after both of the anchor lines above, not between them, so the
# standard after-both-anchors placement is already correct here too.
uus_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$uus_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present update-ui-story "$uus_script_dir" "$@"
unset uus_script_dir

plan_die "update-ui-story: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
