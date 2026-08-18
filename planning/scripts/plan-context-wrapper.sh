#!/usr/bin/env bash
# plan-context-wrapper.sh — source a per-worker variables file, then exec
# plan-context.sh with the remaining arguments.
#
# The wrapper exists so a worker can supply benign context defaults (RUN_ID,
# REVISION, NEXT_ACTION) without every caller having to export them. It writes
# nothing and holds no shared state; the caller owns the short-lived file.
#
# Usage:
#   plan-context-wrapper.sh <variables-file> <plan-context.sh arguments...>
#   plan-context-wrapper.sh --help
#
# Exit codes: 64 bad invocation, 66 variables file missing.

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <variables-file> <plan-context.sh arguments...>
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in -h|--help) usage 0 ;; esac
[ "$#" -ge 2 ] || usage

variables_file="$1"
shift
[ -f "$variables_file" ] || { printf 'variables file not found: %s\n' "$variables_file" >&2; exit 66; }

# The caller owns this short-lived per-worker file; it carries only benign
# context defaults, and this wrapper writes no shared state.
# shellcheck disable=SC1090
source "$variables_file"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$script_dir/plan-context.sh" "$@"
