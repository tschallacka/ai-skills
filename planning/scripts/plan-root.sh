#!/usr/bin/env bash
# MODE: PROD
# Resolve (and, on first use in a project, choose) the plans root.
# This is the single decision point behind PLANS_ROOT so that the first plan
# created in a project asks the human where plans should live, then remembers
# the answer for every later plan in that project.
#
# Usage:
#   plan-root.sh resolve [directory]      # print the resolved plans root
#   plan-root.sh project-root [directory] # print the project (git) root if any
#
# Rules (in priority order):
#   1. If PLANS_ROOT is already exported, it wins (no prompt; automation).
#   2. If <project>/.plans exists and is "consistent" (its .env records that
#      .plans as the plans root), return it as the default (no prompt).
#   3. If a directory matching a global format for this project already exists
#      (<global>/tsch-ai-skills/plans/<owner>/<repo> or <global>/tsch-ai-skills/plans/<user>/<projectdir>, with <global> = ${XDG_CONFIG_HOME:-~/.config}), return it as
#      the recognized root (no prompt). No marker file is written; recognition
#      is purely by matching the directory format.
#   4. Otherwise this is the first plan in the project: when run on an
#      interactive terminal ask the human whether to store globally under
#      the tsch-ai-skills XDG home or in the project's ./.plans; when non-interactive, default to
#      project storage and print a note.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
pr_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$pr_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present plan-root "$pr_script_dir" "$@"
unset pr_script_dir

plan_die "plan-root: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
