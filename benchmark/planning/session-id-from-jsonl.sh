#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <worker.jsonl>" >&2
    exit 64
fi

sed -nE \
    -e 's/.*"type"[[:space:]]*:[[:space:]]*"thread.started".*"thread_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
    -e 's/.*"thread_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
    -e 's/.*"threadId"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
    -e 's/.*"conversation_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
    -e 's/.*"session_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
    "$1" | head -1
