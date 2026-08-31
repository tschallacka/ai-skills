#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
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
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-crypt-lib.sh"

# rjq is invoked by name from these helpers, so the directory holding it has to
# be on PATH before any of them run. plan_bin_dir answers where that is for an
# install and for a development tree alike; it replaces a hand-rolled copy of
# the platform table that also counted parent directories, and got a different
# answer once the library build moved this block two levels shallower (B95).
rjq_dir="$(plan_bin_dir)" || rjq_dir=''
if [ -n "$rjq_dir" ] && { [ -x "$rjq_dir/rjq" ] || [ -x "$rjq_dir/rjq.exe" ]; }; then
    PATH="$rjq_dir:$PATH"
    export PATH
fi

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
