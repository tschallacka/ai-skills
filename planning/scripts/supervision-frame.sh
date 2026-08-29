#!/usr/bin/env bash
# MODE: PROD
# supervision-frame.sh — bounded supervision-frame emitter + grant log.
#
# Every subagent under Willie's supervision ends by writing one bounded
# "supervision frame" (fixed shape, strict byte budget, footer-overwriting the
# previous frame) instead of pushing its raw log. Willie reads only the latest
# frame and pulls deeper only on exception (see monitor-read.sh). A frame must
# stay within FRAME_BUDGET bytes (default 2048); `write` refuses an over-budget
# frame and `check` enforces the same bound for the reader.
#
# Usage:
#   supervision-frame.sh write <frame-file> --subagent NAME --persona ID \
#       --status ok|blocked|escalated|out-of-bounds [--read-discipline ok|violated] \
#       [--wholesale-reads N] [--skill-loaded none|NAME] [--needs-escalation none|CASE] \
#       [--grant-requested none|COMMAND] [--verdict TEXT]
#   supervision-frame.sh grant <grant-log-file> <subagent> <persona> \
#       --case TEXT --command TEXT     # appends case + command, NEVER reasoning
#   supervision-frame.sh show  <frame-file>           # print the latest frame
#   supervision-frame.sh check <frame-file> <budget>  # exit 64 if over budget
#   supervision-frame.sh --help

# NOTE: `usage` below prints lines 1-20 of this file as the help text, so the
# docblock above MUST stay within the first 20 lines (CODE-STYLE.md section 2).
# Anything added here goes below this comment, never into the docblock.

set -euo pipefail
export LC_ALL=C

FRAME_BUDGET="${FRAME_BUDGET:-2048}"

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

frame_write() {
    # NB: `shift 2>/dev/null` — the `2>` is an fd-2 redirect, NOT a count, so
    # this intentionally shifts by 1 (drops the captured leading frame-file).
    local frame_file="${1:-}"; shift 2>/dev/null || true
    local subagent="" persona="" status="" read_discipline=ok wholesale_reads=0 skill_loaded=none needs_escalation=none grant_requested=none verdict=""
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --subagent) [ "$#" -ge 2 ] || usage; subagent="$2"; shift 2 ;;
            --persona) [ "$#" -ge 2 ] || usage; persona="$2"; shift 2 ;;
            --status) [ "$#" -ge 2 ] || usage; status="$2"; shift 2 ;;
            --read-discipline) [ "$#" -ge 2 ] || usage; read_discipline="$2"; shift 2 ;;
            --wholesale-reads) [ "$#" -ge 2 ] || usage; wholesale_reads="$2"; shift 2 ;;
            --skill-loaded) [ "$#" -ge 2 ] || usage; skill_loaded="$2"; shift 2 ;;
            --needs-escalation) [ "$#" -ge 2 ] || usage; needs_escalation="$2"; shift 2 ;;
            --grant-requested) [ "$#" -ge 2 ] || usage; grant_requested="$2"; shift 2 ;;
            --verdict) [ "$#" -ge 2 ] || usage; verdict="$2"; shift 2 ;;
            *) usage ;;
        esac
    done
    if [ -z "$frame_file" ] || [ -z "$subagent" ] || [ -z "$persona" ] || [ -z "$status" ]; then
        usage
    fi
    case "$status" in
        ok|blocked|escalated|out-of-bounds) ;;
        *) printf 'supervision-frame: invalid status "%s" (ok|blocked|escalated|out-of-bounds)\n' "$status" >&2; exit 64 ;;
    esac
    local tmp
    # A named template so a leaked temp is identifiable as this script's.
    tmp="$(mktemp "${TMPDIR:-/tmp}/supervision-frame.XXXXXX")"
    # RETURN, not EXIT: clearing it below cannot discard a caller's EXIT handler.
    trap 'rm -f "$tmp"' RETURN
    {
        printf 'subagent: %s\n' "$subagent"
        printf 'persona: %s\n' "$persona"
        printf 'status: %s\n' "$status"
        printf 'read_discipline: %s\n' "$read_discipline"
        printf 'wholesale_reads: %s\n' "$wholesale_reads"
        printf 'skill_loaded: %s\n' "$skill_loaded"
        printf 'needs_escalation: %s\n' "$needs_escalation"
        printf 'grant_requested: %s\n' "$grant_requested"
        printf 'verdict: %s\n' "$verdict"
    } > "$tmp"
    local size
    # PORTABILITY(wc-padding): strip blanks before interpolating the count.
    size="$(wc -c < "$tmp" | tr -d ' ')"
    if [ "$size" -gt "$FRAME_BUDGET" ]; then
        printf 'supervision-frame: frame %s exceeds byte budget %s (is %s)\n' "$subagent" "$FRAME_BUDGET" "$size" >&2
        return 64
    fi
    mkdir -p "$(dirname "$frame_file")"
    mv "$tmp" "$frame_file"
    trap - RETURN
}

frame_grant() {
    # grant <log-file> <subagent> <persona> --case TEXT --command TEXT
    [ "$#" -ge 3 ] || usage
    local log_file="$1" subagent="$2" persona="$3"; shift 3
    local frame_case="" command=""
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --case) [ "$#" -ge 2 ] || usage; frame_case="$2"; shift 2 ;;
            --command) [ "$#" -ge 2 ] || usage; command="$2"; shift 2 ;;
            *) usage ;;
        esac
    done
    if [ -z "$frame_case" ] || [ -z "$command" ]; then
        usage
    fi
    mkdir -p "$(dirname "$log_file")"
    printf 'grant\t%s\t%s\t%s\t%s\t%s\n' \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$subagent" "$persona" "$frame_case" "$command" >> "$log_file"
}

frame_show() {
    [ "$#" -eq 1 ] || usage
    local frame_file="$1"
    [ -f "$frame_file" ] || { printf 'supervision-frame: no frame at %s\n' "$frame_file" >&2; return 66; }
    cat "$frame_file"
}

frame_check() {
    [ "$#" -eq 2 ] || usage
    local frame_file="$1" budget="$2"
    [ -f "$frame_file" ] || { printf 'supervision-frame: no frame at %s\n' "$frame_file" >&2; return 66; }
    local size
    # PORTABILITY(wc-padding): strip blanks before interpolating the count.
    size="$(wc -c < "$frame_file" | tr -d ' ')"
    if [ "$size" -gt "$budget" ]; then
        printf 'supervision-frame: frame over budget %s (is %s)\n' "$budget" "$size" >&2
        return 64
    fi
    printf 'ok: %s bytes (budget %s)\n' "$size" "$budget"
}

subcommand="${1:-}"; shift 2>/dev/null || true  # shifts by 1 (fd-2 redirect, intended)
case "$subcommand" in
    -h|--help) usage 0 ;;
    write) frame_write "$@" ;;
    grant) frame_grant "$@" ;;
    show) frame_show "$@" ;;
    check) frame_check "$@" ;;
    *) usage ;;
esac
