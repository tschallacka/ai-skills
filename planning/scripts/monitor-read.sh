#!/usr/bin/env bash
# monitor-read.sh — gated, bounded, paginated monitor reader (pull-on-exception).
#
# Willie (the end-user-facing monitor, maintainer persona) supervises personas
# by reading ONLY each subagent's bounded supervision frame (written by
# supervision-frame.sh), never raw logs. This reader serves those frames
# bounded and paginated so a green frame costs ~zero context and Willie pulls
# deeper only when a frame flags escalated/out-of-bounds/blocked.
#
# Usage:
#   monitor-read.sh show <frame-file>                      # bounded frame text
#   monitor-read.sh status <frame-file>                    # one-line: subagent,status
#   monitor-read.sh summary <dir>                          # all frames under <dir>
#   monitor-read.sh grants <grant-log> [--last N]          # grant log (case+command)
#   monitor-read.sh verify <frame-file>                    # fail-closed identity
#   monitor-read.sh --help
#
# Gating: Willie is the maintainer. Reading a frame requires ROLE_ID resolving
# to `maintainer`; non-maintainer callers are refused (fail closed). Budget is
# enforced per frame via supervision-frame.sh check.

# NOTE: `usage` below prints lines 1-20 of this file as the help text, so the
# docblock above MUST stay within the first 20 lines (CODE-STYLE.md section 2).
# Anything added here goes below this comment, never into the docblock.

set -euo pipefail
export LC_ALL=C

FRAME_BUDGET="${FRAME_BUDGET:-2048}"

# PORTABILITY(readlink-f): a symlinked skill dir (~/.claude/skills/planning)
# is the common case, so this must resolve for real or the maintainer is
# locked out of the monitor.
resolve_symlink() {
    local path="$1" target hops=0
    while [ -L "$path" ]; do
        hops=$((hops + 1))
        [ "$hops" -le 40 ] || break     # cycle guard: give up, do not spin
        target="$(readlink "$path")"
        case "$target" in
            /*) path="$target" ;;
            *) path="$(dirname "$path")/$target" ;;
        esac
    done
    printf '%s\n' "$path"
}

script_dir="$(cd "$(dirname "$(resolve_symlink "${BASH_SOURCE[0]}")")" && pwd)"

usage() {
    local rc="${1:-64}"
    awk 'NR == 1 { next }
         /^#/ {
             sub(/^#[[:space:]]?/, "")
             if ($0 ~ /^----[[:space:]]*(quoted:|end quoted)/) next
             print; next
         }
         { exit }' "$0"
    exit "$rc"
}

require_maintainer() {
    # Fail-closed: only the maintainer may read frames. Resolving through the
    # shared persona resolver is what makes both the canonical id and the
    # canonical name acceptable here; unknown or unset identities are refused.
    local resolved
    resolved="$(ROLE_ID="${ROLE_ID:-}" "$BASH" -c 'set -euo pipefail; [ -f "$1" ] && source "$1"; resolve_id "${ROLE_ID:-UNSET}"' _ "$script_dir/role-context.sh" 2>/dev/null || true)"
    if [ "$resolved" != maintainer ]; then
        printf 'monitor-read: FAIL-CLOSED identity: only the maintainer (Willie) may read supervision frames; got ROLE_ID="%s"\n' "${ROLE_ID:-}" >&2
        exit 64
    fi
}

monitor_show() {
    [ "$#" -eq 1 ] || usage
    require_maintainer
    local frame_file="$1"
    [ -f "$frame_file" ] || { printf 'monitor-read: no frame at %s\n' "$frame_file" >&2; return 66; }
    "$BASH" "$script_dir/supervision-frame.sh" check "$frame_file" "$FRAME_BUDGET" >/dev/null || {
        printf 'monitor-read: frame %s over budget; refusing to load\n' "$frame_file" >&2; return 64
    }
    cat "$frame_file"
}

monitor_status() {
    [ "$#" -eq 1 ] || usage
    require_maintainer
    local frame_file="$1"
    [ -f "$frame_file" ] || { printf 'monitor-read: no frame at %s\n' "$frame_file" >&2; return 66; }
    awk -F': ' '/^(subagent|status):/{printf "%s=%s ",$1,$2} END{print ""}' "$frame_file"
}

monitor_summary() {
    [ "$#" -eq 1 ] || usage
    require_maintainer
    local dir="$1"
    [ -d "$dir" ] || { printf 'monitor-read: no frames dir %s\n' "$dir" >&2; return 66; }
    for f in "$dir"/*; do
        [ -f "$f" ] || continue
        printf '%s: ' "$(basename "$f")"
        monitor_status "$f"
    done
}

monitor_grants() {
    local log_file="" last=20
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --last) [ "$#" -ge 2 ] || usage; last="$2"; shift 2 ;;
            -*) usage ;;
            *)
                [ -z "$log_file" ] || usage
                log_file="$1"
                shift
                ;;
        esac
    done
    require_maintainer
    [ -n "$log_file" ] || usage
    [ -f "$log_file" ] || { printf 'monitor-read: no grant log at %s\n' "$log_file" >&2; return 66; }
    tail -n "$last" "$log_file"
}

monitor_verify() {
    [ "$#" -eq 1 ] || usage
    require_maintainer
    local frame_file="$1"
    [ -f "$frame_file" ] || { printf 'monitor-read: no frame at %s\n' "$frame_file" >&2; return 66; }
    local status_line
    status_line="$(awk -F': ' '/^status:/{print $2}' "$frame_file")"
    case "$status_line" in
        ok) printf 'green: %s\n' "$status_line" ;;
        escalated|out-of-bounds|blocked) printf 'PULL-ON-EXCEPTION: status=%s\n' "$status_line" ;;
        *) printf 'monitor-read: unknown status "%s"\n' "$status_line" >&2; return 64 ;;
    esac
}

subcommand="${1:-}"; shift 2>/dev/null || true  # shifts by 1 (fd-2 redirect, intended)
case "$subcommand" in
    -h|--help) usage 0 ;;
    show) monitor_show "$@" ;;
    status) monitor_status "$@" ;;
    summary) monitor_summary "$@" ;;
    grants) monitor_grants "$@" ;;
    verify) monitor_verify "$@" ;;
    *) usage ;;
esac
