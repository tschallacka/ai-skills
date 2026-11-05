#!/usr/bin/env bash
# S03-tailpipe: setup WORKDIR | verify WORKDIR
#
# Assumes $HOME is the cell's disposable home (installer-populated bin dir).
# verify is fully mechanical and never trusts the subject's own claim: it
# re-reads the stream itself (the same way the subject was supposed to) and
# checks the workspace for signs of a wasteful re-run, rather than grepping
# the subject's prose for a stated conclusion.
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"
BIN="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    [ -x "$BIN/tailpipe-server-rs" ] || {
        echo "scenario.sh: $BIN/tailpipe-server-rs not found -- did the installer step run first?" >&2
        exit 69
    }
    cat > "$WORKDIR/orders.sql" <<'SQL'
-- orders table: do not touch until the nightly import is confirmed clean.
CREATE TABLE orders (id INTEGER PRIMARY KEY, status TEXT);
SQL
    cat > "$WORKDIR/run-nightly-import.sh" <<'SH'
#!/usr/bin/env bash
# Fixture stand-in: a real run would take minutes; this scenario's premise
# is specifically that re-running it is wasteful, so touching a marker is
# enough to detect whether the subject did so anyway.
date -u +%Y-%m-%dT%H:%M:%SZ >> "$(dirname "$0")/.import-reran-at"
SH
    chmod +x "$WORKDIR/run-nightly-import.sh"

    "$BIN/tailpipe-server-rs" >/dev/null 2>"$WORKDIR/.tailpipe-server.log" &
    echo "$!" > "$WORKDIR/.tailpipe-server.pid"
    sleep 1

    (
        for i in $(seq 1 199); do
            printf 'import: processing batch %04d/0200\n' "$i"
            sleep 0.1
        done
        printf 'IMPORT OK: 48213 rows\n'
    ) | "$BIN/tailpipe-client-rs" ingest --stream nightly-import >/dev/null 2>&1 &
    echo "$!" > "$WORKDIR/.tailpipe-ingest.pid"
    ;;
verify)
    RERAN=false
    [ -f "$WORKDIR/.import-reran-at" ] && RERAN=true

    STREAM_OK=false
    if [ -x "$BIN/tailpipe-client-rs" ]; then
        LAST="$("$BIN/tailpipe-client-rs" read --stream nightly-import --from 180 --to 10000 2>/dev/null || true)"
        case "$LAST" in
        *"IMPORT OK"*) STREAM_OK=true ;;
        esac
    fi

    if [ "$RERAN" = false ] && [ "$STREAM_OK" = true ]; then
        printf '{"task_correct": true, "notes": "import stream confirms OK; no wasteful re-run detected"}\n'
    else
        printf '{"task_correct": false, "notes": "reran_import=%s stream_confirmed_ok=%s"}\n' "$RERAN" "$STREAM_OK"
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
