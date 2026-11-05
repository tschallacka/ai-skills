#!/usr/bin/env bash
# MODE: DEV
# pre-release-check -- everything RELEASE.md's checklist needs verified
# before tagging a release, in one command, plus a printed checklist for
# what it cannot verify on its own.
#
# Usage:
#   pre-release-check.sh           the gates (see the compiled binary's --help)
#   pre-release-check.sh --full    also run ./run-tests.sh
#   pre-release-check.sh --help

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
prc_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$prc_script_dir/planning/scripts/plan-core-lib.sh"
plan_exec_compiled_binary_if_present pre-release-check "$prc_script_dir" "$@"
unset prc_script_dir

plan_die "pre-release-check: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
