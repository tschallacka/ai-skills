#!/usr/bin/env bash
# MODE: DEV
# Binary serve contract: printed endpoint is live before the first request.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
binary="$repo/target/debug/plan-overview"
fixture="$repo/planning/tests/fixtures/overview/navigation"
cargo build --manifest-path "$repo/src/plan-overview/Cargo.toml" --offline >/dev/null
port_file="$(mktemp)"
err_file="$(mktemp)"
trap 'rm -f "$port_file" "$err_file"; kill "$pid" 2>/dev/null || true' EXIT
OVERVIEW_NOW=fixed "$binary" --plan-dir "$fixture" --serve --port 0 >"$port_file" 2>"$err_file" &
pid=$!
port=''
for _ in 1 2 3 4 5 6 7 8 9 10; do
    [ -s "$port_file" ] && { port="$(cut -d: -f2 "$port_file")"; break; }
    sleep 0.1
done
case "$port" in ''|*[!0-9]*) t_fail 'serve did not print a numeric port' ;; esac
if command -v curl >/dev/null 2>&1 && [ -n "$port" ]; then
    t_assert_contains 'serve answers root immediately' '<!doctype html>' "$(curl -fsS "http://127.0.0.1:$port/")"
    t_assert_contains 'serve answers state endpoint' 'identity' "$(curl -fsS "http://127.0.0.1:$port/state.json")"
else
    printf '%s\n' 'test-overview-serve: curl unavailable; endpoint assertions skipped' >&2
fi
t_end
