#!/usr/bin/env bash
# MODE: PROD
# run-adversary-probe.sh — reusable adversarial-review probe.
#
# Materializes the versioned adversary-probe fixture (planning/tests/fixtures/
# adversary-probe) to a working directory, initializes the gated-reader
# snapshot, sanity-checks that every entry id the probe relies on is served by
# the CURRENT reader, and prints the exact spawn prompt for a fresh adversarial
# reviewer.
#
# Usage:
#   run-adversary-probe.sh [<working-dir>]
#   run-adversary-probe.sh --help
#
# The working copy (default under the planning-agent temp dir) is where a
# reviewer writes its verdict; the committed fixture is never mutated.
#
# Exit codes: 1 = the materialized probe is not usable with the current reader;
# 66 = the committed fixture is missing.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
rap_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$rap_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present run-adversary-probe "$rap_script_dir" "$@"
unset rap_script_dir

plan_die "run-adversary-probe: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
