#!/usr/bin/env bash
# MODE: DEV
# Smoke-check the SHIPPED, packaged editor server and client (the
# x86_64-unknown-linux-musl release artifacts under ai-text-editor/bin/) as
# one runnable flow -- not their behavior in general, which
# src/ai-text-editor/tests/cli_flow.rs (4800+ lines) and tcp_flow.rs (440+
# lines) already prove exhaustively against a dev-profile binary built
# straight from source via `env!("CARGO_BIN_EXE_ai-text-editor")`.
#
# T145 goal 25, W138: this file used to duplicate a large fraction of
# cli_flow.rs's/tcp_flow.rs's own assertions (open/insert/read/search/paging/
# external-change/TCP-auth-rotation), all against the SAME source compiled a
# different way. That duplication provided no coverage those two files did
# not already provide -- a musl-cross-compiled release binary and a
# native-target dev binary run the identical Rust source, so a real
# regression in any of those BEHAVIORS would already be caught there. What
# only THIS file can catch is a genuinely packaging-specific regression: a
# stale or mismatched binary actually shipped in ai-text-editor/bin/, a
# musl-cross-compilation or release-profile-only breakage (a linking issue,
# a panic=abort/strip difference, a musl libc quirk), or the two transports
# (the default local socket, and TCP -- the Windows fallback per SKILL.md)
# failing to even START on the shipped artifact. So this narrows to exactly
# that: build, start, open, edit, read back, and cleanly close, on BOTH
# transports, against the real shipped binaries -- a packaging smoke test,
# not a behavior suite. See this goal's own step 08 investigation for the
# full assertion-by-assertion comparison this narrowing is based on.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
target="$repo_root/ai-text-editor/bin/x86_64-unknown-linux-musl"
server="$target/ai-text-editor-server"
client="$target/ai-text-editor"

if [ ! -x "$server" ] || [ ! -x "$client" ]; then
    printf 'test-ai-text-editor: UNCONFIGURED (shipped Linux binaries are absent)\n'
    exit 0
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/ai-text-editor-test.XXXXXX")"
runtime="$scratch/runtime"
metadata="$scratch/metadata"
mkdir -p "$runtime" "$metadata"
file="$scratch/document.txt"
session="$scratch/session.json"
printf 'alpha\nbeta\n' > "$file"
server_output="$scratch/server-output"
server_pid=""
tcp_pid=""
cleanup() {
    if [ -n "$server_pid" ]; then
        kill "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    if [ -n "$tcp_pid" ]; then
        kill "$tcp_pid" 2>/dev/null || true
        wait "$tcp_pid" 2>/dev/null || true
    fi
    rm -rf "$scratch"
}
trap cleanup EXIT INT TERM
contains() {
    case "$1" in
        *"$2"*) return 0 ;;
        *) return 1 ;;
    esac
}

export XDG_RUNTIME_DIR="$runtime"
export TSCH_AI_EDITOR_METADATA_DIR="$metadata"

# ---- the default (local socket) transport: open, edit, read, save, close --
"$server" start --file "$file" >"$server_output" 2>&1 &
server_pid="$!"
ready=0
open_output=""
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    if open_output="$($client open --file "$file" --save-session-token "$session" 2>/dev/null)"; then
        ready=1
        break
    fi
    sleep 0.1
done
[ "$ready" -eq 1 ] || { sed -n '1,120p' "$server_output" >&2; exit 1; }
contains "$open_output" '"revision": 0'

insert_output="$("$client" insert --file "$file" --offset 5 --text '!' --expected-revision 0 --session-token "$session")"
contains "$insert_output" '"revision": 1'

read_output="$("$client" read --file "$file" --session-token "$session")"
contains "$read_output" 'alpha!'

"$client" save --file "$file" --expected-revision 1 --session-token "$session" >/dev/null
"$client" close --file "$file" --journal-action clean --session-token "$session" >/dev/null
wait "$server_pid" 2>/dev/null || true
server_pid=""

# ---- the TCP transport (the Windows autostart fallback per SKILL.md) -----
tcp_file="$scratch/tcp-document.txt"
printf 'tcp-content\n' > "$tcp_file"
auth_file="$scratch/tcp-auth-token"
printf 'secret\n' > "$auth_file"
chmod 600 "$auth_file"
tcp_output="$scratch/tcp-server-output"
tcp_session="$scratch/tcp-session.json"
"$server" start --file "$tcp_file" --tcp 127.0.0.1:0 --auth-token-file "$auth_file" >"$tcp_output" 2>&1 &
tcp_pid="$!"
tcp_endpoint=""
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    if [ -s "$tcp_output" ]; then
        tcp_endpoint="$(sed -n 's/.*"endpoint":"\([^"]*\)".*/\1/p' "$tcp_output")"
        [ -n "$tcp_endpoint" ] && break
    fi
    sleep 0.1
done
[ -n "$tcp_endpoint" ] || { sed -n '1,120p' "$tcp_output" >&2; exit 1; }
tcp_open="$($client open --endpoint "$tcp_endpoint" --auth-token secret --save-session-token "$tcp_session")"
contains "$tcp_open" '"mode": "text_utf8"'
tcp_read="$($client read --endpoint "$tcp_endpoint" --auth-token secret --session-token "$tcp_session")"
contains "$tcp_read" 'tcp-content'
"$client" close --endpoint "$tcp_endpoint" --auth-token secret --session-token "$tcp_session" --journal-action clean >/dev/null
wait "$tcp_pid" 2>/dev/null || true
tcp_pid=""

printf 'test-ai-text-editor: PASS\n'
