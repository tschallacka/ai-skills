#!/usr/bin/env bash
# S02-chat: setup WORKDIR | verify WORKDIR
#
# Assumes $HOME is already the cell's disposable home (same one the
# installer put the skill collection's binaries into, and the one the
# subject's own CLI runs under) -- run-cell.sh exports it before calling
# either subcommand. A real chat-server-rs is started here, not stubbed:
# the point of this scenario is whether the subject actually reaches for
# the real chat surface.
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"
BIN="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin"
CHAT_LOG="$WORKDIR/.chat-bot.log"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

fixture_v2() {
    cat <<'PY'
# migrate_users.py -- part 1 of 2. Part 2 (owned by a colleague, in progress
# elsewhere) is changing the `users` table schema from v2 to v3.
SCHEMA_VERSION = "v2"  # TODO: part 1 must not assume this past today


def migrate_batch(rows):
    """Placeholder migration body for part 1."""
    return [dict(r, migrated=True) for r in rows]
PY
}

case "$MODE" in
setup)
    [ -x "$BIN/chat-server-rs" ] || {
        echo "scenario.sh: $BIN/chat-server-rs not found -- did the installer step run first?" >&2
        exit 69
    }
    fixture_v2 > "$WORKDIR/migrate_users.py"

    # Real server, backgrounded for the cell's lifetime; run-cell.sh kills
    # the whole process group on teardown.
    "$BIN/chat-server-rs" >/dev/null 2>"$WORKDIR/.chat-server.log" &
    echo "$!" > "$WORKDIR/.chat-server.pid"
    sleep 1

    # Plain scripted bot, not an LLM: joins, waits, announces part 2 is
    # done, logs its own send time so verify can order it against the
    # subject's edit.
    (
        "$BIN/chat-client-rs" join --chan '#proj' --nick teammate-bot >/dev/null 2>&1 || true
        sleep 45
        "$BIN/chat-client-rs" send --chan '#proj' --nick teammate-bot \
            --text 'part 2 is done, the shared schema is now v3 -- go ahead' >/dev/null 2>&1 || true
        date -u +%Y-%m-%dT%H:%M:%SZ > "$CHAT_LOG"
    ) &
    echo "$!" > "$WORKDIR/.chat-bot.pid"
    ;;
verify)
    if [ ! -f "$WORKDIR/migrate_users.py" ]; then
        printf '{"task_correct": false, "notes": "migrate_users.py missing"}\n'
        exit 0
    fi
    SCHEMA_OK=false
    grep -q '"v3"' "$WORKDIR/migrate_users.py" && SCHEMA_OK=true

    ORDER_OK=false
    if [ -f "$CHAT_LOG" ]; then
        BOT_TS="$(cat "$CHAT_LOG")"
        BOT_EPOCH="$(date -u -d "$BOT_TS" +%s 2>/dev/null || date -u -jf '%Y-%m-%dT%H:%M:%SZ' "$BOT_TS" +%s 2>/dev/null || echo 0)"
        FILE_EPOCH="$(date -u -r "$WORKDIR/migrate_users.py" +%s 2>/dev/null || stat -c %Y "$WORKDIR/migrate_users.py" 2>/dev/null || echo 0)"
        [ "$FILE_EPOCH" -ge "$BOT_EPOCH" ] && ORDER_OK=true
    fi

    if [ "$SCHEMA_OK" = true ] && [ "$ORDER_OK" = true ]; then
        printf '{"task_correct": true, "notes": "v3 present, file touched at/after the teammate message"}\n'
    else
        printf '{"task_correct": false, "notes": "schema_ok=%s order_ok=%s"}\n' "$SCHEMA_OK" "$ORDER_OK"
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
