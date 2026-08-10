#!/usr/bin/env bash

set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
    echo "Usage: $0 <name> [--parallel|--sequential] [--versions]" >&2
    exit 64
fi

NAME="$1"
shift
MODE="--sequential"
INTERACTIVE_VERSIONS=0
for ARG in "$@"; do
    case "$ARG" in
        --parallel|--sequential)
            MODE="$ARG"
            ;;
        --versions)
            INTERACTIVE_VERSIONS=1
            ;;
        *)
            echo "Unknown option: $ARG" >&2
            echo "Usage: $0 <name> [--parallel|--sequential] [--versions]" >&2
            exit 64
            ;;
    esac
done

if [[ ! "$NAME" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    echo "Name must start with a letter or number and contain only letters, numbers, '.', '_' or '-'" >&2
    exit 64
fi

case "$MODE" in
    --parallel|--sequential) ;;
    *)
        echo "Unknown execution mode: $MODE" >&2
        echo "Usage: $0 <name> [--parallel|--sequential] [--versions]" >&2
        exit 64
        ;;
esac

RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)-$NAME"
export RUN_ID
ARGS=("$NAME" "/tmp/$RUN_ID" "$MODE")
if [ "$INTERACTIVE_VERSIONS" -eq 1 ]; then
    ARGS+=(--versions)
fi
exec "$(dirname "$0")/run-benchmark.sh" "${ARGS[@]}"
