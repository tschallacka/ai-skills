#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 3 ]; then
    echo "Usage: $(basename "$0") <goal-directory> <step-name> <incomplete|in-progress|completed>" >&2
    exit 64
fi

goal_dir="$1"
step_name="$2"
requested_status="$3"
progress_file="$goal_dir/progress.md"

case "$requested_status" in
    incomplete)
        status='💤 incomplete'
        ;;
    in-progress|in_progress)
        status='⏳ in progress'
        ;;
    completed)
        status='✅ completed'
        ;;
    *)
        echo "Unknown status: $requested_status" >&2
        echo "Use: incomplete, in-progress, or completed" >&2
        exit 64
        ;;
esac

if [ ! -f "$progress_file" ]; then
    echo "Progress file not found: $progress_file" >&2
    exit 66
fi

temporary_file="${progress_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT

awk -F'|' -v wanted_step="$step_name" -v replacement="$status" '
    BEGIN { found = 0 }
    /^\|/ {
        step = $3
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", step)
        if (step == wanted_step) {
            sub(/\|[[:space:]]*[^|]*[[:space:]]*\|[[:space:]]*$/, "| " replacement " |")
            found++
        }
    }
    { print }
    END { if (found != 1) exit 1 }
' "$progress_file" > "$temporary_file" || {
    echo "Step row not found exactly once: $step_name" >&2
    exit 1
}

mv "$temporary_file" "$progress_file"
trap - EXIT

"$(dirname "$0")/update-progress.sh" "$goal_dir"
