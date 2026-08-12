#!/usr/bin/env bash

set -euo pipefail

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <name> [--parallel|--sequential] [--iterative|--fresh-review] [--revisions tag[,tag...]] [--versions]" >&2
    exit 64
fi

NAME="$1"
shift
MODE="--sequential"
INTERACTIVE_VERSIONS=0
REVIEW_MODE="fresh-review"
REVISIONS=""
while [ "$#" -gt 0 ]; do
    ARG="$1"
    case "$ARG" in
        --parallel|--sequential)
            MODE="$ARG"
            ;;
        --versions)
            INTERACTIVE_VERSIONS=1
            ;;
        --iterative|--fresh-review)
            candidate_mode="${ARG#--}"
            if [ "$REVIEW_MODE" != "fresh-review" ] || { [ "$ARG" = "--fresh-review" ] && [ -n "$REVISIONS" ]; }; then
                echo "Review mode may be specified only once." >&2
                exit 64
            fi
            REVIEW_MODE="$candidate_mode"
            ;;
        --revisions)
            shift
            [ "$#" -gt 0 ] || { echo "--revisions requires tag[,tag...] as its next argument." >&2; exit 64; }
            REVISIONS="$1"
            [ -n "$REVISIONS" ] || { echo "Revision list may not be empty." >&2; exit 64; }
            ;;
        --revisions=*)
            REVISIONS="${ARG#--revisions=}"
            [ -n "$REVISIONS" ] || { echo "Revision list may not be empty." >&2; exit 64; }
            ;;
        *)
            echo "Unknown option: $ARG" >&2
            echo "Usage: $0 <name> [--parallel|--sequential] [--versions]" >&2
            exit 64
            ;;
    esac
    shift
done

if [[ ! "$NAME" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    echo "Name must start with a letter or number and contain only letters, numbers, '.', '_' or '-'" >&2
    exit 64
fi

case "$MODE" in
    --parallel|--sequential) ;;
    *)
        echo "Unknown execution mode: $MODE" >&2
        echo "Usage: $0 <name> [--parallel|--sequential] [--iterative|--fresh-review] [--revisions tag[,tag...]] [--versions]" >&2
        exit 64
        ;;
esac

RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)-$NAME"
export RUN_ID
ARGS=("$NAME" "/tmp/$RUN_ID" "$MODE" "--$REVIEW_MODE")
[ -n "$REVISIONS" ] && ARGS+=(--revisions "$REVISIONS")
if [ "$INTERACTIVE_VERSIONS" -eq 1 ]; then
    ARGS+=(--versions)
fi
exec "$(dirname "$0")/run-benchmark.sh" "${ARGS[@]}"
