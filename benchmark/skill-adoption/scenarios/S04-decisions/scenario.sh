#!/usr/bin/env bash
# S04-decisions: setup WORKDIR | verify WORKDIR
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    cat > "$WORKDIR/client.py" <<'PY'
import urllib.request


def fetch(url):
    """No resilience against transient failures yet -- one failed request is one failure."""
    with urllib.request.urlopen(url) as resp:
        return resp.read()
PY
    git -C "$WORKDIR" init -q 2>/dev/null || true
    git -C "$WORKDIR" add -A 2>/dev/null || true
    git -C "$WORKDIR" -c user.email=a@b.c -c user.name=setup commit -q -m "initial" 2>/dev/null || true
    ;;
verify)
    if [ ! -f "$WORKDIR/client.py" ]; then
        printf '{"task_correct": false, "notes": "client.py missing"}\n'
        exit 0
    fi
    HAS_RETRY=false
    grep -Eiq 'retry|backoff' "$WORKDIR/client.py" && HAS_RETRY=true

    # Whether the agent actually avoided blocking (vs. asking inline and
    # waiting) is a transcript property, not a filesystem one -- this
    # script asserts only the mechanically checkable half (the code landed)
    # and leaves the "didn't block" half to adoption_signals plus a human
    # reading observer_notes, rather than faking a filesystem heuristic for
    # it.
    if [ "$HAS_RETRY" = true ]; then
        printf '{"task_correct": true, "notes": "retry/backoff present in client.py"}\n'
    else
        printf '{"task_correct": false, "notes": "no retry/backoff found in client.py"}\n'
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
