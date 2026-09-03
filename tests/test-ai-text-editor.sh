#!/usr/bin/env bash
# MODE: DEV
# Verify the shipped editor server and short-lived client as one runnable flow.
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
large_pid=""
cleanup() {
    if [ -n "$server_pid" ]; then
        kill "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    if [ -n "$tcp_pid" ]; then
        kill "$tcp_pid" 2>/dev/null || true
        wait "$tcp_pid" 2>/dev/null || true
    fi
    if [ -n "$large_pid" ]; then
        kill "$large_pid" 2>/dev/null || true
        wait "$large_pid" 2>/dev/null || true
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
editor() {
    "$client" "$@" --session-token "$session"
}

export XDG_RUNTIME_DIR="$runtime"
export TSCH_AI_EDITOR_METADATA_DIR="$metadata"
export TSCH_AI_EDITOR_AGENT="editor-integration-agent"
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
contains "$open_output" '"mode": "TextUtf8"'
contains "$open_output" '"tab_uuid": "'
capabilities_output="$($client capabilities --session-token "$session")"
contains "$capabilities_output" '"protocol_version": 1'
contains "$capabilities_output" '"search_preview_matches": 4'
contains "$capabilities_output" '"revision_required_methods"'
if editor insert --file "$file" --offset 0 --text 'unsafe' >"$scratch/missing-revision" 2>&1; then
    exit 1
fi
grep -Fq 'revision_required' "$scratch/missing-revision"
agent_open="$($client open --agent "$TSCH_AI_EDITOR_AGENT")"
contains "$agent_open" '"session_token"'
unset TSCH_AI_EDITOR_AGENT
second_file="$scratch/second-document.txt"
second_session="$scratch/second-session.json"
printf 'second-tab\n' > "$second_file"
endpoint="$(sed -n 's/.*"endpoint":"\([^"]*\)".*/\1/p' "$server_output")"
[ -n "$endpoint" ]
discovery="$(sed -n 's/.*"discovery":"\([^"]*\)".*/\1/p' "$server_output")"
[ -n "$discovery" ]
grep -Fq '"status":"active"' "$discovery"
grep -Fq '"pid":' "$discovery"
grep -Fq '"generation":' "$discovery"

invalid_session="$scratch/invalid-session.json"
printf '{"endpoint":"%s","session_token":"invalid-token"}\n' "$endpoint" > "$invalid_session"
if "$client" open --session-token "$invalid_session" >"$scratch/invalid-session-output" 2>&1; then
    exit 1
fi
grep -Fq 'session_unauthorized' "$scratch/invalid-session-output"

second_open="$($client open --endpoint "$endpoint" --file "$second_file" --save-session-token "$second_session")"
contains "$second_open" 'second-document.txt'
second_read="$($client read --endpoint "$endpoint" --session-token "$second_session")"
contains "$second_read" 'second-tab'
original_read="$($client read --file "$file" --session-token "$session")"
contains "$original_read" 'alpha'

# A second startup must reuse the validated per-tab SQLite index.
"$client" close --endpoint "$endpoint" --session-token "$second_session" --journal-action clean >/dev/null
editor close --file "$file" --journal-action preserve >/dev/null
wait "$server_pid" 2>/dev/null || true
server_pid=""
"$server" start --file "$file" >"$server_output" 2>&1 &
server_pid="$!"
reopened=""
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    if reopened="$($client open --file "$file" --save-session-token "$session" 2>/dev/null)"; then
        break
    fi
    sleep 0.1
done
contains "$reopened" '"index_loaded": true'

insert_output="$(editor insert --file "$file" --offset 5 --text '!' --expected-revision 0)"
contains "$insert_output" '"revision": 1'

read_output="$(editor read --file "$file")"
contains "$read_output" 'alpha!'

wrapped_cursor="$(editor cursor --file "$file" --line 1 --column 4 --wrap-width 3)"
contains "$wrapped_cursor" '"visual": {'
contains "$wrapped_cursor" '"line": 2'
visual_cursor="$(editor cursor --file "$file" --line 2 --column 1 --wrap-width 3 --visual)"
contains "$visual_cursor" '"line": 1'
contains "$visual_cursor" '"column": 4'
editor cursor --id 7 --line 2 --column 0 --file "$file" >/dev/null
cursor_context="$(editor read --cursor-id 7 --before 1 --after 0 --file "$file")"
contains "$cursor_context" '"start_line": 1'
contains "$cursor_context" 'beta'

search_output="$(editor search --file "$file" --mode exact_text --query beta)"
contains "$search_output" '"count": 1'
contains "$search_output" '"pager_key"'
if editor search --file "$file" --mode regex_rust --query '[' >"$scratch/invalid-search" 2>&1; then
    exit 1
fi
grep -Fq 'search_invalid' "$scratch/invalid-search"
pager_key="$(printf '%s\n' "$search_output" | sed -n 's/.*"pager_key": "\([^"]*\)",/\1/p')"
[ -n "$pager_key" ]

# Preserve the journal across a server restart so the recovered revision still
# matches the persisted result set. Paging must reload the result from SQLite.
editor save --file "$file" --expected-revision 1 >/dev/null
editor close --file "$file" --journal-action preserve >/dev/null
wait "$server_pid" 2>/dev/null || true
server_pid=""
"$server" start --file "$file" >"$server_output" 2>&1 &
server_pid="$!"
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    if "$client" open --file "$file" --save-session-token "$session" >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done
page_output="$(editor page --file "$file" --pager-key "$pager_key" --limit 1)"
contains "$page_output" '"count": 1'
contains "$page_output" '"contents": "beta"'
editor insert --file "$file" --offset 0 --text 'X' --expected-revision 1 >/dev/null
if editor page --file "$file" --pager-key "$pager_key" --limit 1 >"$scratch/stale-page" 2>&1; then
    exit 1
fi
grep -Fq 'stale_result' "$scratch/stale-page"
historical_page="$(editor page --file "$file" --pager-key "$pager_key" --limit 1 --historical)"
contains "$historical_page" '"source_revision": 1'
contains "$historical_page" '"stale": true'

printf 'outside\n' > "$file"
if editor open --file "$file" >"$scratch/external-output" 2>&1; then
    exit 1
fi
grep -Fq 'external_change' "$scratch/external-output"

backup_output="$(editor resolve --file "$file" --action backup)"
contains "$backup_output" '"resolved": "backup"'
[ -f "$file.back" ]
reload_output="$(editor resolve --file "$file" --action reload)"
contains "$reload_output" '"history_event": "external_reload"'

if editor close --file "$file" >"$scratch/close-prompt" 2>&1; then
    exit 1
fi
grep -Fq 'journal_close_decision_required' "$scratch/close-prompt"
editor close --file "$file" --journal-action clean >/dev/null
wait "$server_pid" 2>/dev/null || true
server_pid=""
remaining_metadata="$(find "$metadata" -name 'tab-*.sqlite' -type f -print)"
if [ -n "$remaining_metadata" ]; then
    printf 'unexpected metadata after original clean close: %s\n' "$remaining_metadata" >&2
    sed -n '1,120p' "$server_output" >&2
    exit 1
fi

# TCP is the Windows transport fallback. It must challenge the client before
# accepting a request, and a failed proof must not reach the editor handler.
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
contains "$tcp_open" '"mode": "TextUtf8"'
if "$client" open --endpoint "$tcp_endpoint" --auth-token wrong >"$scratch/tcp-auth-failure" 2>&1; then
    exit 1
fi
grep -Fq 'authentication_failed' "$scratch/tcp-auth-failure"
tcp_read="$($client read --endpoint "$tcp_endpoint" --auth-token secret --session-token "$tcp_session")"
contains "$tcp_read" 'tcp-content'
printf 'rotated\n' > "$auth_file"
if "$client" read --endpoint "$tcp_endpoint" --auth-token secret --session-token "$tcp_session" >"$scratch/rotated-auth-failure" 2>&1; then
    exit 1
fi
grep -Fq 'authentication_failed' "$scratch/rotated-auth-failure"
tcp_rotated="$($client read --endpoint "$tcp_endpoint" --auth-token rotated --session-token "$tcp_session")"
contains "$tcp_rotated" 'tcp-content'
"$client" close --endpoint "$tcp_endpoint" --auth-token rotated --session-token "$tcp_session" --journal-action clean >/dev/null
wait "$tcp_pid" 2>/dev/null || true
tcp_pid=""

large_file="$scratch/large-document.txt"
session="$scratch/large-session.json"
printf 'first\nneedle\nlast\n' > "$large_file"
large_output="$scratch/large-server-output"
"$server" start --file "$large_file" --large-threshold-bytes 1 >"$large_output" 2>&1 &
large_pid="$!"
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    if large_open="$($client open --file "$large_file" --save-session-token "$session" 2>/dev/null)"; then
        break
    fi
    sleep 0.1
done
contains "$large_open" '"index_complete": false'
contains "$large_open" '"through_line": 1'
large_context="$(editor read --line 2 --before 1 --after 1 --file "$large_file")"
contains "$large_context" '"start_line": 1'
contains "$large_context" '"end_line": 3'
contains "$large_context" 'needle'
if editor search --file "$large_file" --mode exact_text --query needle >"$scratch/unbounded-large-search" 2>&1; then
    exit 1
fi
grep -Fq 'large_search_range_required' "$scratch/unbounded-large-search"
large_search="$(editor search --file "$large_file" --mode exact_text --query needle --range-start-line 1 --range-end-line 3)"
contains "$large_search" '"count": 1'
contains "$large_search" '"start_line": 1'
contains "$large_search" '"end_line": 3'
large_regex="$(editor search --file "$large_file" --mode regex_rust --query 'n.*dle' --range-start-line 1 --range-end-line 3)"
contains "$large_regex" '"count": 1'
large_fuzzy="$(editor search --file "$large_file" --mode fuzzy_subsequence --query nedle --range-start-line 1 --range-end-line 3)"
contains "$large_fuzzy" '"count": 1'
large_pager_key="$(printf '%s\n' "$large_search" | sed -n 's/.*"pager_key": "\([^"]*\)",/\1/p')"
[ -n "$large_pager_key" ]
large_wildcard="$(editor search --file "$large_file" --mode wildcard --query needle --range-start-line 1 --range-end-line 3)"
large_wildcard_key="$(printf '%s\n' "$large_wildcard" | sed -n 's/.*"pager_key": "\([^"]*\)",/\1/p')"
[ -n "$large_wildcard_key" ]
[ "$large_pager_key" != "$large_wildcard_key" ]
large_page="$(editor page --file "$large_file" --pager-key "$large_pager_key" --limit 1)"
contains "$large_page" '"contents": "needle"'
large_bytes="$(editor search --file "$large_file" --mode exact_bytes --query-base64 bmVlZGxl --range-start-byte 0 --range-end-byte 18)"
contains "$large_bytes" '"byte_start": 6'
contains "$large_bytes" '"contents_base64": "bmVlZGxl"'
large_index="$(editor index --file "$large_file" --granularity 2)"
contains "$large_index" '"complete": true'
large_index_page="$(editor index --file "$large_file" --granularity 2 --offset 1 --limit 1)"
contains "$large_index_page" '"block_offset": 1'
contains "$large_index_page" '"returned_blocks": 1'
printf 'externally-reloaded\n' > "$large_file"
if editor open --file "$large_file" >"$scratch/large-external-output" 2>&1; then
    exit 1
fi
grep -Fq 'external_change' "$scratch/large-external-output"
large_backup="$(editor resolve --file "$large_file" --action backup)"
contains "$large_backup" '"large_file": true'
[ -f "$large_file.back" ]
large_reload="$(editor resolve --file "$large_file" --action reload)"
contains "$large_reload" '"history_event": "external_reload"'
contains "$large_reload" '"index_complete": false'
editor close --file "$large_file" --journal-action clean >/dev/null
wait "$large_pid" 2>/dev/null || true
large_pid=""
remaining_metadata="$(find "$metadata" -name 'tab-*.sqlite' -type f -print)"
if [ -n "$remaining_metadata" ]; then
    printf 'unexpected metadata after clean close: %s\n' "$remaining_metadata" >&2
    sed -n '1,120p' "$large_output" >&2
    exit 1
fi

printf 'test-ai-text-editor: PASS\n'
