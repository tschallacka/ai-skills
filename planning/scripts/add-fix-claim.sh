#!/usr/bin/env bash
# MODE: PROD
# add-fix-claim.sh — record one fix-key claim in a plan's fixes.md.
#
# fixes.md had five readers and no writer. The fixer was expected to produce it,
# yet SKILL.md forbids hand-authoring a plan artifact, so the only way to satisfy
# the fix-key gate was to break that rule. This is the writer.
#
# Usage:
#   add-fix-claim.sh [--plan-dir] <plan-directory> --finding <AR-NN> \
#       --work-unit <WNN> --key <hex>
#   add-fix-claim.sh --help
#
# One claim per call, appended as `finding_id \t work_unit \t key`, which is the
# shape verify-fix-keys.sh reads.
#
# The key is checked for shape and for being gated, never derived: deriving it
# needs the minting session's secret, and a fixer that could reach that secret
# could mint its own keys. Cryptographic verification stays in
# verify-fix-keys.sh, run by a session that is not the minting one.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
afc_bin_pref_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$afc_bin_pref_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present add-fix-claim "$afc_bin_pref_script_dir" "$@"
unset afc_bin_pref_script_dir

plan_die "add-fix-claim: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
