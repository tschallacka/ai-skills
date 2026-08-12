#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ]; then
    printf 'Usage: %s <variables-file> <plan-context.sh arguments...>\n' "$(basename "$0")" >&2
    exit 64
fi

variables_file="$1"
shift
[ -f "$variables_file" ] || { printf 'variables file not found: %s\n' "$variables_file" >&2; exit 66; }

# The caller owns this short-lived per-worker file. It may provide benign
# context defaults (RUN_ID, REVISION, NEXT_ACTION); no shared state is written
# by this wrapper.
# shellcheck disable=SC1090
source "$variables_file"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$script_dir/plan-context.sh" "$@"
