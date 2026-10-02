#!/usr/bin/env bash
# MODE: PROD
# verify-target.sh — statically check that a work unit's target surface renders.
#
# The plan gate requires evidence that a template/block/layout target actually
# renders before work is planned against it (a file existing is not evidence).
# This helper performs the static half of that check for one work unit:
#   1. the target file exists, and whose it is (core / module / theme);
#   2. no layout removes the block that renders it (<referenceBlock remove="true">);
#   3. no layout re-points it to another template (setTemplate / re-registered
#      <block ... template>);
#   4. for a module template, whether any theme overrides it.
#
# Usage:
#   The plan directory may be given positionally or as --plan-dir <path>.
#   verify-target.sh [--plan-dir] <plan-directory> <WNN> [--repo <repository-root>]
#   verify-target.sh --help
#
# Exit codes: 0 = target present with no static counter-evidence; 1 = target
# missing, a layout removes its block, or the check could not run at all;
# 64 = usage/plan error.
#
# It is advisory: PASS is evidence of existence and of no static layout
# counter-evidence, not a proof the surface renders in a live browser. Record
# the outcome in the plan's discovery unit; steps 2-3 are the ones a
# theme-override search misses, so this tool checks them explicitly.
#
# --repo defaults to the current directory. The unit's File/Scope columns are
# read from work-unit-inventory.md; layout scan roots are the repository's
# view/*/layout and layout directories (Magento-style) plus etc/view.xml.
#
# What runs is decided by the TARGET, not the type column: a render-surface file
# gets checks 1-4 under any type, any other file gets 1 and 4, and a unit naming
# no target — or a surface with no block name — fails closed, because the check
# cannot run. No type exits 0 unchecked.
#
# Output discipline (CODE-STYLE.md section 10): stdout carries exactly one
# result line (a PASS naming which checks ran); every per-check OK, WARN and FAIL
# diagnostic goes to stderr, so `x="$(verify-target.sh …)"` yields the verdict.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. This script's own plan_hoist_plan_dir call
# runs after both of the anchor lines above, not between them, so the
# standard after-both-anchors placement is already correct here too. Per
# AR-22, the compiled binary itself parses --plan-dir=VAL and --repo=VAL
# directly (src/verify-target/src/main.rs).
vt_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$vt_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present verify-target "$vt_script_dir" "$@"
unset vt_script_dir

plan_die "verify-target: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
