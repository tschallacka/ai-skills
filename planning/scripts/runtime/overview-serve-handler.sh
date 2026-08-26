#!/usr/bin/env bash
# MODE: PROD
# overview-serve-handler.sh — HTTP responses for the served overview.
# Called by the runtime-specific server with:
#   $1 = plan directory
# Outputs the response body to stdout based on the ROUTE environment variable.
set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
plan_dir="${PLAN_DIR:?}"
route="${ROUTE:-/}"

# HTTP line in, body out; socat EXEC connects stdin/stdout to the socket.
read -r method route http_version || exit 0
route="${route%%\?*}"

render_to_temp() { # PATH_OUT — render once; the renderer writes a file
    "$script_dir/../render-plan-overview.sh" "$plan_dir" --serve \
        --out "$1" >/dev/null 2>&1
}

section_of() { # ID — the id="ID" element's HTML from a fresh render,
    # sliced from its opening tag to just before the next section anchor.
    local want_id="$1" fresh_file
    fresh_file="$(mktemp "${TMPDIR:-/tmp}/overview-section.XXXXXX")"
    render_to_temp "$fresh_file" || { rm -f "$fresh_file"; return 1; }
    sed -nE "/id=\"$want_id\"/,/id=\"(identity-panel|step-details|tests-panel|coverage-panel|findings-panel|dep-graph|narr)\"|<\/main>|<\/body>/p" "$fresh_file" | sed '$d'
    rm -f "$fresh_file"
}

case "$route" in
    /state.json|/state)
        printf 'Content-Type: application/json; charset=utf-8\r\n\r\n'
        "$script_dir/../overview-state.sh" "$plan_dir"
        ;;
    /sections/*)
        sec="${route#/sections/}"
        case "$sec" in
            identity-panel|step-details|tests-panel|coverage-panel|findings-panel|dep-graph|narr) ;;
            *) printf 'Status: 404\r\nContent-Type: text/plain\r\n\r\nNot found\n'; exit 0 ;;
        esac
        body="$(section_of "$sec")"
        [ -n "$body" ] || { printf 'Status: 404\r\nContent-Type: text/plain\r\n\r\nNot found\n'; exit 0; }
        printf 'Content-Type: text/html; charset=utf-8\r\n\r\n'
        printf '%s\n' "$body"
        ;;
    /|/index.html)
        page="$(mktemp "${TMPDIR:-/tmp}/overview-page.XXXXXX")"
        render_to_temp "$page" || { rm -f "$page"; printf 'Status: 500\r\n\r\n'; exit 0; }
        printf 'Content-Type: text/html; charset=utf-8\r\n\r\n'
        cat "$page"
        rm -f "$page"
        ;;
    *)
        printf 'Status: 404\r\nContent-Type: text/plain\r\n\r\nNot found: %s\n' "$route"
        ;;
esac
