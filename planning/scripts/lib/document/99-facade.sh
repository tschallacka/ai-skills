#!/usr/bin/env bash
# The façade every planning script sources. It pulls in the sibling libraries so
# that `source plan-document-lib.sh` provides the same symbols it always did:
# 40-plus scripts source this path, and the split must be invisible to them.
#
# Sorted last in the group (99-) so every definition above it exists before the
# initialisation block runs.
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-core-lib.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-table-lib.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-progress-lib.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-map-lib.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-inventory-lib.sh"

# ── Load-time initialisation ─────────────────────────────────────────────────
# Guarded: this library is sourced more than once per process, and re-running it
# would reset plan_error_count and record plan_cleanup as its own "prior" handler.
if [ -z "${PLAN_DOCUMENT_LIB_INITIALISED:-}" ]; then
    PLAN_DOCUMENT_LIB_INITIALISED=1
    plan_error_count=0
    plan_tmp_files=()
    plan_prior_exit_trap="$(trap -p EXIT)"
    trap plan_cleanup EXIT INT TERM
fi
