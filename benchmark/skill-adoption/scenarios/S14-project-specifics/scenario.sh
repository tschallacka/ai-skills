#!/usr/bin/env bash
# S14-project-specifics: setup WORKDIR | verify WORKDIR
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    mkdir -p "$WORKDIR/.agents/knowledge"
    cat > "$WORKDIR/.agents/knowledge/deviations-example.md" <<'MD'
# Deviations from defaults

- `build.sh` requires `NODE_OPTIONS=--max-old-space-size=4096` or it fails
  silently under memory pressure and still exits 0 -- a plain exit-code
  check on this build is not trustworthy on its own.
MD
    rm -f "$WORKDIR/.build-ok"
    cat > "$WORKDIR/build.sh" <<'SH'
#!/usr/bin/env bash
# Deliberately always exits 0 -- the trap is that success is NOT what the
# exit code says.
if [[ "${NODE_OPTIONS:-}" == *"max-old-space-size=4096"* ]]; then
    echo "build succeeded"
    touch "$(dirname "$0")/.build-ok"
else
    echo "build succeeded"  # lies, on purpose -- this is the fixture's point
fi
exit 0
SH
    chmod +x "$WORKDIR/build.sh"
    ;;
verify)
    ACTUALLY_OK=false
    [ -f "$WORKDIR/.build-ok" ] && ACTUALLY_OK=true

    CLAIMED_OK="unknown"
    if [ -f "$WORKDIR/.subject-conclusion.txt" ]; then
        if grep -qiE 'succe?eded|passed|built (ok|successfully)|build is (good|fine|ok)' "$WORKDIR/.subject-conclusion.txt"; then
            CLAIMED_OK=true
        elif grep -qiE 'fail|did not (actually )?(succeed|build)|broken|not (actually )?(ok|fine|successful)' "$WORKDIR/.subject-conclusion.txt"; then
            CLAIMED_OK=false
        fi
    fi

    if [ "$CLAIMED_OK" = "unknown" ]; then
        printf '{"task_correct": false, "notes": "could not determine subject'"'"'s claim from .subject-conclusion.txt (actually_ok=%s)"}\n' "$ACTUALLY_OK"
    elif [ "$CLAIMED_OK" = "$ACTUALLY_OK" ]; then
        printf '{"task_correct": true, "notes": "claim matched reality (actually_ok=%s)"}\n' "$ACTUALLY_OK"
    else
        printf '{"task_correct": false, "notes": "claimed_ok=%s but actually_ok=%s -- trusted the lying exit code"}\n' "$CLAIMED_OK" "$ACTUALLY_OK"
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
