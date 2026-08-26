#!/usr/bin/env bash
# MODE: DEV
# test-overview-serve — T43f: both delivery modes agree, every runtime rung
# serves identical routes, the no-runtime rung refuses with exit 69, and a
# clean stop leaves nothing behind. The served flavor carries no reload call.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

scripts="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
plan_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/.plans/harden-plan-data-parsing"
[ -d "$plan_dir" ] || plan_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/.plans/overview-review-and-live-serve"
[ -d "$plan_dir" ] || t_fail "no self-hosted plan fixture under .plans/"
work="$(mktemp -d "${TMPDIR:-/tmp}/overview-serve.XXXXXX")"
trap 'rm -rf "$work"' EXIT

fetch() { # URL -> body on stdout (curl when present, python3 otherwise)
    if command -v curl >/dev/null 2>&1; then
        curl -fsS --max-time 15 "$1" 2>/dev/null || true
    else
        python3 - "$1" <<'PYEOF'
import sys, urllib.request
try:
    sys.stdout.write(urllib.request.urlopen(sys.argv[1], timeout=15).read().decode())
except Exception:
    pass
PYEOF
    fi
}

norm_state() { # JSON — generatedAt varies per render; strip it for equality
    sed 's/"generatedAt":"[^"]*"/"generatedAt":"X"/'
}

# ---- both modes render from one source: state extractor feeds the pin ----
want_state="$(OVERVIEW_NOW=fixed "$BASH" "$scripts/overview-state.sh" "$plan_dir" | norm_state)"

start_bg() { # NAME CMD... -> pid file + port file polled for 6s
    local name="$1"; shift
    ("$@" >"$work/port.$name" 2>"$work/err.$name" & echo $! >"$work/pid.$name")
    local i
    for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
        [ -s "$work/port.$name" ] && break
        sleep 0.3
    done
}

rung_port=""

serve_checks() { # NAME PORT — the identical-route contract every rung meets
    local name="$1" port="$2"
    [ -n "$port" ] || { t_fail "$name never reported a port"; return; }
    local got html sec
    got="$(fetch "http://127.0.0.1:$port/state.json" | norm_state)"
    t_assert_eq "$name /state.json equals the extractor" "$got" "$want_state"
    html="$(fetch "http://127.0.0.1:$port/")"
    t_assert_contains "$name / serves the artifact" '<!DOCTYPE html>' "$html"
    case "$html" in *'location.reload('*) t_fail "$name served flavor calls location.reload" ;; esac
    sec="$(fetch "http://127.0.0.1:$port/sections/identity-panel")"
    t_assert_contains "$name /sections slices identity-panel" 'id="identity-panel"' "$sec"
    sec="$(fetch "http://127.0.0.1:$port/sections/no-such-panel")"
    case "$sec" in *identity-panel*|*tests-panel*) t_fail "$name served a bogus section id" ;; esac
}

# ---- python3 rung ----------------------------------------------------------
if command -v python3 >/dev/null 2>&1; then
    start_bg py python3 "$scripts/runtime/overview-server.py" "$plan_dir"
    rung_port="$(head -1 "$work/port.py")"
    serve_checks python3 "$rung_port"
    kill "$(cat "$work/pid.py")" 2>/dev/null || true; rm -f "$work/pid.py"
    sleep 0.4
    # Exit status carries the verdict: 0 = still accepting (bad), 3 = refused.
    if python3 - "$rung_port" <<'PYEOF'
import socket, sys
s = socket.socket()
s.settimeout(1)
try:
    s.connect(("127.0.0.1", int(sys.argv[1])))
except Exception:
    sys.exit(3)
finally:
    s.close()
PYEOF
    then t_fail "python3 listener survived the stop"; fi
else
    printf 'overview-serve: python3 absent; rung skipped\n' >&2
fi

# ---- node rung ---------------------------------------------------------------
NODE_BIN=""
if command -v node >/dev/null 2>&1; then NODE_BIN=node
elif command -v nodejs >/dev/null 2>&1; then NODE_BIN=nodejs; fi
if [ -n "$NODE_BIN" ]; then
    start_bg node "$NODE_BIN" "$scripts/runtime/overview-server.js" "$plan_dir"
    rung_port="$(head -1 "$work/port.node")"
    serve_checks node "$rung_port"
    kill "$(cat "$work/pid.node")" 2>/dev/null || true; rm -f "$work/pid.node"
else
    printf 'overview-serve: node absent; rung skipped\n' >&2
fi

# ---- perl rung (core modules only) -------------------------------------------
if command -v perl >/dev/null 2>&1; then
    start_bg pl perl "$scripts/runtime/overview-server.pl" "$plan_dir"
    rung_port="$(head -1 "$work/port.pl")"
    serve_checks perl "$rung_port"
    kill "$(cat "$work/pid.pl")" 2>/dev/null || true; rm -f "$work/pid.pl"
else
    printf 'overview-serve: perl absent; rung skipped\n' >&2
fi

# ---- socat rung (explicit port; bash handler) --------------------------------
if command -v socat >/dev/null 2>&1; then
    sport=$(( 20000 + RANDOM % 20000 ))
    (PLAN_DIR="$plan_dir" nohup socat "TCP-LISTEN:$sport,fork,reuseaddr,bind=127.0.0.1" \
        "SYSTEM:$scripts/runtime/overview-serve-handler.sh,stderr" \
        >"$work/socat.log" 2>&1 & echo $! >"$work/pid.socat")
    sleep 1
    serve_checks socat "$sport"
    kill "$(cat "$work/pid.socat")" 2>/dev/null || true; rm -f "$work/pid.socat"
else
    printf 'overview-serve: socat absent; rung skipped\n' >&2
fi

# ---- no-runtime rung: exit 69 naming the capability ---------------------------
# A PATH that has every tool the script needs EXCEPT the four runtimes: build
# it by symlinking the standard PATH bins, skipping python3/node/perl/socat.
nort="$work/nort"; mkdir -p "$nort"
_IFS="$IFS"; IFS=':'
for d in $PATH; do
    IFS="$_IFS"
    [ -d "$d" ] || continue
    for b in "$d"/*; do
        [ -f "$b" ] && [ -x "$b" ] || continue
        base="${b##*/}"
        case "$base" in python3|python|node|nodejs|perl|socat) continue ;; esac
        [ -e "$nort/$base" ] || ln -sf "$b" "$nort/$base"
    done
    IFS=':'
done
IFS="$_IFS"
out="$(PATH="$nort" "$BASH" "$scripts/overview-serve.sh" "$plan_dir" 2>&1 || true)"
case "$out" in
    *'no suitable runtime'*python3*|*'no suitable runtime'*socat*) : ;;
    *) t_fail "no-runtime refusal did not name the capability: $out" ;;
esac
rc=0; PATH="$nort" "$BASH" "$scripts/overview-serve.sh" "$plan_dir" >/dev/null 2>&1 || rc=$?
t_assert_eq "no-runtime exits 69" "$rc" 69

# ---- file mode stays a snapshot; served mode swaps in place --------------------
OVERVIEW_NOW=fixed "$BASH" "$scripts/render-plan-overview.sh" "$plan_dir" --out "$work/file.html" >/dev/null 2>&1
OVERVIEW_NOW=fixed "$BASH" "$scripts/render-plan-overview.sh" "$plan_dir" --serve --out "$work/served.html" >/dev/null 2>&1
t_assert_contains "file mode keeps its meta refresh" 'http-equiv="refresh"' "$(cat "$work/file.html")"
if grep -q 'http-equiv="refresh"' "$work/served.html"; then
    t_fail "served mode kept the meta refresh"
fi
if grep -q 'location.reload(' "$work/served.html"; then
    t_fail "served flavor contains a reload call"
fi
t_assert_contains "served flavor polls sections" 'fetch("/sections/"' "$(cat "$work/served.html")"
t_assert_contains "served flavor carries the revision swap" 'data-rev' "$(cat "$work/served.html")"

t_end
