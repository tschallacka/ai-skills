#!/usr/bin/env bash
# MODE: PROD
# overview-serve-handler.sh — HTTP responses for the served overview.
# Called by the runtime-specific server with:
#   $1 = plan directory
# Outputs the response body to stdout based on the ROUTE environment variable.
set -euo pipefail
export LC_ALL=C

# runtime/ sits two levels below the skill root (planning/scripts/runtime in
# the tree, <skill>/scripts/runtime when installed); resolve the root once and
# descend, so both layouts find the sibling scripts.
skill_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scripts_dir="$skill_root/scripts"
case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64) rjq_dir="$skill_root/bin/x86_64-unknown-linux-musl" ;;
    Linux:aarch64|Linux:arm64) rjq_dir="$skill_root/bin/aarch64-unknown-linux-musl" ;;
    Darwin:x86_64) rjq_dir="$skill_root/bin/x86_64-apple-darwin" ;;
    Darwin:arm64) rjq_dir="$skill_root/bin/aarch64-apple-darwin" ;;
    *) rjq_dir="$skill_root/bin/x86_64-pc-windows-msvc" ;;
esac
if [ -x "$rjq_dir/rjq" ] || [ -x "$rjq_dir/rjq.exe" ]; then
    PATH="$rjq_dir:$PATH"
    export PATH
fi
plan_dir="${PLAN_DIR:?}"
route="${ROUTE:-/}"

# HTTP line in, body out; socat EXEC connects stdin/stdout to the socket.
read -r method route http_version || exit 0
route="${route%%\?*}"

render_to_temp() { # PATH_OUT — render once; the renderer writes a file
    "$scripts_dir/render-plan-overview.sh" "$plan_dir" --serve \
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

respond() { # STATUS TYPE [BODY_FILE] — one honest HTTP/1.0 response
    local status="$1" ctype="$2" body_file="${3:-}" len=0
    [ -n "$body_file" ] && [ -f "$body_file" ] && len="$(wc -c < "$body_file")"
    printf 'HTTP/1.0 %s\r\nContent-Type: %s\r\nContent-Length: %s\r\nConnection: close\r\n\r\n' \
        "$status" "$ctype" "$len"
    [ -n "$body_file" ] && cat "$body_file"
}

case "$route" in
    /state.json|/state)
        body="$(mktemp "${TMPDIR:-/tmp}/ovh-state.XXXXXX")"
        if "$scripts_dir/overview-state.sh" "$plan_dir" > "$body"; then
            respond "200 OK" "application/json; charset=utf-8" "$body"
        else
            respond "500 Internal Server Error" "text/plain"
        fi
        rm -f "$body"
        ;;
    /sections/*)
        sec="${route#/sections/}"
        body="$(mktemp "${TMPDIR:-/tmp}/ovh-sec.XXXXXX")"
        ok=0
        case "$sec" in
            identity-panel|step-details|tests-panel|coverage-panel|findings-panel|dep-graph|narr)
                section_of "$sec" > "$body" && [ -s "$body" ] && ok=1 ;;
        esac
        if [ "$ok" = 1 ]; then
            respond "200 OK" "text/html; charset=utf-8" "$body"
        else
            respond "404 Not Found" "text/plain"
            printf 'Not found\n'
        fi
        rm -f "$body"
        ;;
    /|/index.html)
        page="$(mktemp "${TMPDIR:-/tmp}/overview-page.XXXXXX")"
        if render_to_temp "$page"; then
            respond "200 OK" "text/html; charset=utf-8" "$page"
        else
            respond "500 Internal Server Error" "text/plain"
        fi
        rm -f "$page"
        ;;
    *)
        respond "404 Not Found" "text/plain"
        printf 'Not found: %s\n' "$route"
        ;;
esac
