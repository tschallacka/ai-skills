#!/usr/bin/env bash
# S17-post-implementation-review: setup WORKDIR | verify WORKDIR
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    cat > "$WORKDIR/app.py" <<'PY'
def handle_request(client_id):
    """No rate limiting yet."""
    return {"ok": True, "client_id": client_id}
PY
    ;;
verify)
    if [ ! -f "$WORKDIR/rate_limiter.py" ]; then
        printf '{"task_correct": false, "notes": "rate_limiter.py missing"}\n'
        exit 0
    fi
    RESULT="$(cd "$WORKDIR" && python3 - <<'PY'
import importlib
import sys
import threading

sys.path.insert(0, ".")
try:
    rl = importlib.import_module("rate_limiter")
except Exception as e:
    print("import_error:%s" % e)
    raise SystemExit

candidates = [n for n in dir(rl) if not n.startswith("_")]
limiter = None
for name in candidates:
    obj = getattr(rl, name)
    if hasattr(obj, "allow") or hasattr(obj, "is_allowed") or callable(obj):
        limiter = obj
        break

if limiter is None:
    print("no_usable_api")
    raise SystemExit

def call(client_id):
    try:
        if hasattr(limiter, "allow"):
            return limiter.allow(client_id)
        if hasattr(limiter, "is_allowed"):
            return limiter.is_allowed(client_id)
        return limiter(client_id)
    except TypeError:
        return limiter(client_id="x")

allowed_count = [0]
lock = threading.Lock()

def worker():
    for _ in range(50):
        r = call("same-client")
        if r:
            with lock:
                allowed_count[0] += 1

threads = [threading.Thread(target=worker) for _ in range(10)]
for t in threads:
    t.start()
for t in threads:
    t.join()

print("allowed_count:%d" % allowed_count[0])
PY
)"
    case "$RESULT" in
    import_error*|no_usable_api*)
        printf '{"task_correct": false, "notes": "rate_limiter.py has no usable, importable API: %s"}\n' "$RESULT"
        ;;
    allowed_count:*)
        COUNT="$(printf '%s' "$RESULT" | sed -n 's/^allowed_count:\([0-9]*\)$/\1/p')"
        if [ -n "$COUNT" ] && [ "$COUNT" -le 100 ]; then
            printf '{"task_correct": true, "notes": "concurrency smoke test stayed within budget: %s allowed"}\n' "$COUNT"
        else
            printf '{"task_correct": false, "notes": "concurrency smoke test exceeded the 100/window budget: %s allowed"}\n' "$COUNT"
        fi
        ;;
    *)
        printf '{"task_correct": false, "notes": "could not interpret smoke test output: %s"}\n' "$RESULT"
        ;;
    esac
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
