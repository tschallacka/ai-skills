#!/usr/bin/env bash
# MODE: PROD
# add-planning-bug.sh — record one defect in a plan's planning-bugs.json.
#
# planning-bugs.json had readers in five places and nothing that wrote it, so an
# agent asking for it got exit 66 on every plan that ever existed. This is the
# writer.
#
# Usage:
#   add-planning-bug.sh [--plan-dir] <plan-directory> --id <PB-NN> --title <text> \
#       --reproduce <text> --observed <text> --expected <text> \
#       [--severity blocking|major|minor|cosmetic] [--priority urgent|high|normal|low|someday] \
#       [--status reported|confirmed] [--found-by <text>]
#   add-planning-bug.sh --help
#
# The file follows the bug-report skill's schema, so its rjq recipes read a plan's
# register unchanged. Defects about the work the plan describes belong here; a
# defect in the planning skill itself belongs in the repository's own BUGS.json.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
apb_bin_pref_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$apb_bin_pref_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present add-planning-bug "$apb_bin_pref_script_dir" "$@"
unset apb_bin_pref_script_dir

plan_die "add-planning-bug: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
