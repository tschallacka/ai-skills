#!/usr/bin/env bash
# MODE: PROD
# add-ui-story-links.sh — update one UI story's related work-unit references.
#
# Usage:
#   add-ui-story-links.sh [--plan-dir] <plan-directory> <US-NN> <WNN[,WNN...]>
#   add-ui-story-links.sh --help

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
ausl_bin_pref_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$ausl_bin_pref_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present add-ui-story-links "$ausl_bin_pref_script_dir" "$@"
unset ausl_bin_pref_script_dir

plan_die "add-ui-story-links: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
