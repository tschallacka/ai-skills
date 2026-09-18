#!/usr/bin/env bash
# MODE: PROD
# generate-reviewer.sh — project the marked skill-source.txt sections into
# REVIEWER.md.
#
# Copies each `<!-- REVIEWER_SECTION:START <name> -->` … `:END` block out of the
# authored source into a compact reviewer contract, prefixed with the reviewer
# profile version and the source's SHA-256. The recorded hash is what
# test-reviewer-projection.sh compares, so neither the projection nor the hash
# format may change without regenerating REVIEWER.md.
#
# T87: the source moved from SKILL.md to skill-source.txt when SKILL.md
# became a generated index (generate-skill-docs.sh); the REVIEWER_SECTION
# markers themselves did not move or change shape, so this script's own
# extraction logic is otherwise unchanged.
#
# Usage:
#   generate-reviewer.sh [<skill-directory>] [<output-file>]
#   generate-reviewer.sh --help
#
# Defaults: the skill directory is this script's parent, the output is
# <skill-directory>/REVIEWER.md.
#
# Exit codes: 65 = a reviewer section is missing, duplicated, or empty;
# 66 = the source skill is absent; 69 = no SHA-256 implementation (the
# plan-crypt binary, sha256sum, or shasum).

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
gr_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$gr_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present generate-reviewer "$gr_script_dir" "$@"
unset gr_script_dir

plan_die "generate-reviewer: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
