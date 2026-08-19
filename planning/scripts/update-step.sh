#!/usr/bin/env bash
# update-step.sh — set one step's completion status in its goal's tracker.
#
# Rewrites the step's row in <goal-directory>/progress.md (canonical 4 data
# columns, so awk -F'|' matches the step name in $3 and replaces the trailing
# status cell), then re-derives the goal's bar by invoking update-progress.sh.
# It refuses when the step row is absent or appears more than once, because
# guessing which row to edit would silently corrupt the tracker.
#
# The child's progress line goes to stderr: stdout carries exactly this
# script's own one-line result (CODE-STYLE §10).
#
# Usage:
#   update-step.sh <goal-directory> <step-name> <incomplete|in-progress|completed>
#   update-step.sh --help
#
# Exit codes: 1 the step row is not present exactly once, 64 bad invocation,
# 66 the goal has no progress.md.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <goal-directory> <step-name> <incomplete|in-progress|completed>
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in
    -h|--help) usage 0 ;;
esac
[ "$#" -eq 3 ] || usage

goal_dir="$1"
step_name="$2"
requested_status="$3"
progress_file="$goal_dir/progress.md"

status="$(plan_status_label "$requested_status")" || {
    printf 'Unknown status: %s\n' "$requested_status" >&2
    printf 'Use: incomplete, in-progress, or completed\n' >&2
    exit 64
}

[ -f "$progress_file" ] || plan_die "Progress file not found: $progress_file" 66
plan_git_snapshot "$(dirname "$goal_dir")"

# No `trap - EXIT` release: it would discard the library's cleanup handler (§8).
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
    printf 'Step row not found exactly once: %s\n' "$step_name" >&2
    exit 1
}

mv "$temporary_file" "$progress_file"

# Sibling invocation goes through the BASH_SOURCE-derived script_dir, never
# `dirname "$0"`: the installer copies these scripts into a skill root that
# users symlink, and $0 is then the symlink's directory.
"$script_dir/update-progress.sh" "$goal_dir" >&2

printf 'Updated %s (%s: %s)\n' "$progress_file" "$step_name" "$requested_status"
