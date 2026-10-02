#!/usr/bin/env bash
# MODE: PROD
# chat-interrupt-plugin/hooks/lib.sh -- shared by hooks/pre-tool-use.sh.
#
# Two independent pieces of logic factored out so they can be tested directly
# (same convention as tui-hint-plugin/hooks/lib.sh and
# editor-gate-plugin/hooks/lib.sh): resolving which session's spool to read,
# and telling whether a file was last written too long ago to trust.
#
# No `stat -c` and no `stat -f`: GNU is one, BSD/macOS is the other, and Git
# for Windows' bash carries neither reliably. `find -newer` against a
# backdated reference file compares mtimes without reading one, the same
# trick planning/scripts/plans-board-lib.sh already uses for the identical
# problem (board_last_activity/board_touch_ago).

# chat_interrupt_hook_session PAYLOAD -- the session id: $CLAUDE_CODE_SESSION_ID
# if set, else PAYLOAD's own "session_id" field (the hook's stdin, when the
# harness runs a hook without exporting the variable), reduced to the alphabet
# the bridge names its spool directories with -- so a stray character in
# either source can never address a directory it should not.
chat_interrupt_hook_session() {
    local payload="$1" session="${CLAUDE_CODE_SESSION_ID:-}"
    if [ -z "$session" ]; then
        session="$(printf '%s' "$payload" \
            | sed -n 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)"
    fi
    printf '%s' "$session" | tr -c 'A-Za-z0-9_-' '_'
}

# chat_interrupt_hook_backdate FILE SECONDS -- set FILE's mtime SECONDS in the
# past. `date -d` is GNU, `date -r` works on both GNU and BSD/macOS.
chat_interrupt_hook_backdate() {
    local file="$1" secs="$2" stamp
    stamp="$(date -u -d "@$(( $(date -u +%s) - secs ))" '+%Y%m%d%H%M.%S' 2>/dev/null \
        || date -u -r "$(( $(date -u +%s) - secs ))" '+%Y%m%d%H%M.%S' 2>/dev/null || true)"
    [ -n "$stamp" ] || return 1
    touch -t "$stamp" "$file" 2>/dev/null
}

# chat_interrupt_hook_is_stale FILE SECONDS -- true (exit 0) when FILE is
# missing, or was last written more than SECONDS ago; false (exit 1) when it
# is fresher than that, or when freshness could not be determined at all
# (never nag on a maybe). Used for both a heartbeat gone quiet and a cooldown
# on the reminder itself, which is exactly "is it time to check this again".
chat_interrupt_hook_is_stale() {
    local file="$1" secs="$2" ref
    [ -f "$file" ] || return 0
    ref="$(mktemp "${TMPDIR:-/tmp}/chat-interrupt-stale.XXXXXX" 2>/dev/null)" || return 1
    if chat_interrupt_hook_backdate "$ref" "$secs" \
        && [ -n "$(find "$file" -newer "$ref" 2>/dev/null)" ]; then
        rm -f "$ref"
        return 1
    fi
    rm -f "$ref"
    return 0
}
