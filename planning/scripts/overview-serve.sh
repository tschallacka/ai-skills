#!/usr/bin/env bash
# MODE: PROD
# overview-serve.sh — serve the plan overview live on localhost.
#
# Picks the first available runtime from the same chain chat uses
# (python3 → node → perl → socat+bash handler). Without any of them, refuses
# with exit 69 naming the missing capability and pointing at file mode.
# The served page updates in place from /state.json without reloading.
#
# Usage:
#   overview-serve.sh [--plan-dir] <plan-directory> [--port N]
#   overview-serve.sh --help

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> [--port N]
       ${0##*/} --help

Serves the plan overview on localhost via the first available runtime
(python3, node, perl, or socat driving the bash handler). The page updates
in place from /state.json and /sections/<id> without reloading. Without any
runtime, exit 69 names the missing capability; use
render-plan-overview.sh for a static snapshot instead.
USAGE
    exit "$rc"
}

plan_dir="" want_port=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --port) [ "$#" -ge 2 ] || usage; want_port="$2"; shift 2 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) [ -z "$plan_dir" ] || usage; plan_dir="$1"; shift ;;
    esac
done
[ -n "$plan_dir" ] || usage
case "$want_port" in ''|*[!0-9]*) want_port="" ;; esac

plan_require_directory "$plan_dir"

runtime=""
for r in python3 node perl socat; do
    if command -v "$r" >/dev/null 2>&1; then runtime="$r"; break; fi
done
[ -n "$runtime" ] || {
    printf '%s: no suitable runtime found (need python3, node, perl, or socat).\n' "${0##*/}" >&2
    printf 'Use planning/scripts/render-plan-overview.sh for a static snapshot instead.\n' >&2
    exit 69
}
printf '%s: serving via %s\n' "${0##*/}" "$runtime" >&2

case "$runtime" in
    python3)
        exec python3 "$script_dir/runtime/overview-server.py" "$plan_dir" ${want_port:+"$want_port"}
        ;;
    node)
        exec node "$script_dir/runtime/overview-server.js" "$plan_dir" ${want_port:+"$want_port"}
        ;;
    perl)
        exec perl "$script_dir/runtime/overview-server.pl" "$plan_dir" ${want_port:+"$want_port"}
        ;;
    socat)
        # socat drives the bash handler per connection; an explicit port is
        # required because socat cannot report a chosen one back to us.
        if [ -z "$want_port" ]; then
            printf '%s: the socat rung needs an explicit --port N (it cannot report a chosen port)\n' \
                "${0##*/}" >&2
            exit 64
        fi
        exec socat "TCP-LISTEN:${want_port},fork,reuseaddr,bind=127.0.0.1" \
            "SYSTEM:$script_dir/runtime/overview-serve-handler.sh,stderr"
        ;;
esac
