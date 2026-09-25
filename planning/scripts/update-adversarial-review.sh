#!/usr/bin/env bash
# MODE: PROD
# update-adversarial-review.sh — rewrite the "## Findings" table in a plan's
# adversarial-review.md from CSV rows (ID, Missing or over-broad item, Required
# plan change, Status, Work unit).
#
# Reads CSV from adversarial-review-incoming.md when that file exists (the one
# plan file a reviewer may write, so a reviewer report survives the
# coordinator's context), else from --file PATH, else from stdin
# (heredoc-friendly). It rewrites only Findings; the Verdict is authored by the
# reviewer and flipped to approved via update-plan-content.sh --review-status.
#
# Usage:
#   update-adversarial-review.sh [--plan-dir] <plan-directory> [--file CSV] [--cycle N]
#   update-adversarial-review.sh --help
#
# The Work unit column is mandatory-with-blank-allowed: leave it empty (or N/A)
# for findings that carry no fix key, or name the owning work unit (WNN) to gate
# the finding. Cells must not contain `|`, and input must be LF, not CRLF.
#
# The input is a findings document, not bare rows: title/comment lines starting
# with `#`, blank lines, and one repeated copy of the header row are skipped,
# whatever the source, so a reviewer can paste their whole document.
#
# Nothing is rewritten unless the new rows mint cleanly: the rewrite is minted
# against a throwaway copy first, so a gated-row or openssl refusal leaves
# every plan file byte-identical.
#
# This cycle's own newly-landed Findings table is archived into
# adversarial-review-history.md under a `## Cycle N` heading (--cycle numbers
# it; otherwise the highest recorded number plus one), paired with the
# Review-scope preamble that describes it, so reviewers of later cycles can
# see what earlier ones found. Archiving the same rows twice is a no-op; a
# --cycle that names an already-recorded cycle while holding different rows is
# refused rather than dropping them.
#
# Exit codes: 64 bad invocation, 65 unusable CSV, 66 the plan or its review file
# is missing, 73 --cycle collides with a recorded cycle holding other findings.

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed before this script's own
# plan_hoist_plan_dir call below: that call rewrites a --plan-dir flag into a
# positional argument, and the compiled binary must receive the caller's true
# original argv, not the already-hoisted form. Per AR-19, the compiled binary
# itself parses --plan-dir=<val> directly (src/update-adversarial-review/src/main.rs).
uar_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$uar_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present update-adversarial-review "$uar_script_dir" "$@"
unset uar_script_dir

plan_die "update-adversarial-review: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
