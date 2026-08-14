#!/usr/bin/env bash
# supervision-frame.sh — bounded supervision-frame emitter + grant log.
#
# Every subagent under Willie's supervision ends by writing one bounded
# "supervision frame" (fixed shape, strict byte budget, footer-overwrites the
# previous frame) instead of pushing its raw log. Willie reads only the latest
# frame and pulls deeper only on exception (see monitor-read.sh).
#
# Commands:
#   supervision-frame.sh write  <frame-file> \
#       --subagent NAME --persona ID --status ok|blocked|escalated|out-of-bounds \
#       [--read-discipline ok|violated] [--wholesale-reads N] \
#       [--skill-loaded none|NAME] [--needs-escalation none|CASE] \
#       [--grant-requested none|COMMAND] [--verdict TEXT]
#       # Footer-overwrites: the file holds exactly the latest frame.
#   supervision-frame.sh grant  <grant-log-file> <subagent> <persona> \
#       --case TEXT --command TEXT
#       # Appends one grant line: case + handed command, NEVER reasoning.
#   supervision-frame.sh show   <frame-file>           # print latest frame
#   supervision-frame.sh check  <frame-file> <budget>  # boundedness check (exit 64 if over)
#
# Byte budget: a frame must stay within the declared size (default 2048 bytes);
# `write` refuses an over-budget frame and `check` enforces it for the reader.

set -euo pipefail

FRAME_BUDGET="${FRAME_BUDGET:-2048}"

usage() {
    sed -n '1,22p' "$0" >&2
    exit 64
}

frame_write() {
    # NB: `shift 2>/dev/null` — the `2>` is an fd-2 redirect, NOT a count, so
    # this intentionally shifts by 1 (drops the captured leading frame-file).
    local frame_file="${1:-}"; shift 2>/dev/null || true
    local subagent="" persona="" status="" read_discipline=ok wholesale_reads=0 skill_loaded=none needs_escalation=none grant_requested=none verdict=""
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --subagent) subagent="$2"; shift 2 ;;
            --persona) persona="$2"; shift 2 ;;
            --status) status="$2"; shift 2 ;;
            --read-discipline) read_discipline="$2"; shift 2 ;;
            --wholesale-reads) wholesale_reads="$2"; shift 2 ;;
            --skill-loaded) skill_loaded="$2"; shift 2 ;;
            --needs-escalation) needs_escalation="$2"; shift 2 ;;
            --grant-requested) grant_requested="$2"; shift 2 ;;
            --verdict) verdict="$2"; shift 2 ;;
            *) usage ;;
        esac
    done
    [ -n "$frame_file" ] && [ -n "$subagent" ] && [ -n "$persona" ] && [ -n "$status" ] || usage
    case "$status" in
        ok|blocked|escalated|out-of-bounds) ;;
        *) printf 'supervision-frame: invalid status "%s" (ok|blocked|escalated|out-of-bounds)\n' "$status" >&2; exit 64 ;;
    esac
    local tmp
    tmp="$(mktemp)"
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
    size="$(wc -c < "$tmp")"
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
    local log_file="$1" subagent="$2" persona="$3"; shift 3
    local frame_case="" command=""
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --case) frame_case="$2"; shift 2 ;;
            --command) command="$2"; shift 2 ;;
            *) usage ;;
        esac
    done
    [ -n "$frame_case" ] && [ -n "$command" ] || usage
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
    size="$(wc -c < "$frame_file")"
    if [ "$size" -gt "$budget" ]; then
        printf 'supervision-frame: frame over budget %s (is %s)\n' "$budget" "$size" >&2
        return 64
    fi
    printf 'ok: %s bytes (budget %s)\n' "$size" "$budget"
}

subcommand="${1:-}"; shift 2>/dev/null || true  # shifts by 1 (fd-2 redirect, intended)
case "$subcommand" in
    write) frame_write "$@" ;;
    grant) frame_grant "$@" ;;
    show) frame_show "$@" ;;
    check) frame_check "$@" ;;
    *) usage ;;
esac
