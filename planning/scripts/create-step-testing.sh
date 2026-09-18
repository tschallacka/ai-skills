#!/usr/bin/env bash
# MODE: PROD
# create-step-testing.sh — create or (with --overwrite) replace a step's
# testing companion. Input is validated BEFORE any filesystem change, so a
# rejected call never leaves the plan with the old companion already deleted.
#
# Usage:
#   create-step-testing.sh <goal-directory> <step-name> <verification-instructions>
#   create-step-testing.sh <goal-directory> <step-name> <verification-instructions> --overwrite
#   create-step-testing.sh --help
#
# The positional instructions are rendered as a numbered §2.x section under
# "## Automated tests"; separate paragraphs with "\n" escapes or real newlines
# (multi-paragraph companions are supported and are what reviewers actually
# proofread; every paragraph gets its own § 2.N label).
#
# --browser, --backend and --manual add the other verification sections. A
# section number is fixed per section name, not by position: automated tests are
# always §2.x, browser §3.x, backend §4.x, manual §5.x, whichever sections a
# companion happens to carry. That is what makes `update-plan-content.sh -ss`
# able to address them -- and until these flags existed it advertised three
# sections that no helper could create.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
cst_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$cst_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present create-step-testing "$cst_script_dir" "$@"
unset cst_script_dir

plan_die "create-step-testing: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
