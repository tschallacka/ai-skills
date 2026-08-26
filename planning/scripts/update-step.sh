#!/usr/bin/env bash
# MODE: PROD
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
                   [--repo-root DIR --unit WNN [--since GIT-REF]]
       ${0##*/} --help

With --unit and --repo-root, completion runs a mechanical atomicity check:
the unit's declared target (inventory row) is compared against the changed
files visible to git; matching evidence ticks the step file's boxes and extra
paths are recorded as a VIOLATION annotation on the third box.
USAGE
    exit "$rc"
}

case "${1:-}" in
    -h|--help) usage 0 ;;
esac
[ "$#" -ge 3 ] || usage

goal_dir="$1"
step_name="$2"
requested_status="$3"

repo_root=""
unit_id=""
since_ref="HEAD"
shift 3
while [ "$#" -gt 0 ]; do
    case "$1" in
        --repo-root) repo_root="${2:-}"; shift 2 ;;
        --unit)      unit_id="${2:-}"; shift 2 ;;
        --since)     since_ref="${2:-}"; shift 2 ;;
        *) plan_die "unknown option: $1" 64 ;;
    esac
done
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

# --- mechanical atomicity check (W05) --------------------------------------
[ "$requested_status" = "completed" ] || exit 0
[ -n "$repo_root" ] && [ -n "$unit_id" ] || exit 0
plan_dir_root="$(cd "$goal_dir/.." && pwd)"
inventory="$plan_dir_root/work-unit-inventory.md"
[ -f "$inventory" ] || { printf 'atomicity: no inventory at %s\n' "$inventory" >&2; exit 0; }
declared_target=""
while IFS= read -r row; do
    [ "$(plan_table_cell "$row" 2)" = "$unit_id" ] || continue
    declared_target=$(plan_table_cell "$row" 4)
    break
done < "$inventory"
if [ -z "$declared_target" ] || [ "$declared_target" = "N/A" ]; then
    printf 'atomicity: %s has no file target; boxes left for manual confirmation\n' "$unit_id" >&2
    exit 0
fi
# Bookkeeping exemption: the plan's own tracker and step files change as part
# of completing any unit; they are never scope violations.
changed="$(git -C "$repo_root" diff --name-only "$since_ref" \
    | grep -v "^$(printf '%s' "${plan_dir_root#"$repo_root"/}")/" | sort -u)"
extra="$(printf '%s\n' "$changed" | grep -Fxv "$declared_target" || true)"
step_file="$goal_dir/steps/$step_name.md"
[ -f "$step_file" ] || { printf 'atomicity: step file missing: %s\n' "$step_file" >&2; exit 0; }
python3 - "$step_file" <<PYATOMIC
import re, sys
path = sys.argv[1]
text = open(path).read()
extras = """$extra""".strip()
violation = (" VIOLATION: also touched " + ", ".join(sorted(extras.splitlines()))) if extras else ""
boxes = [
    "This step owns exactly one inventory work unit.",
    "No other file, symbol, test target, or verification flow changes here.",
    "Any follow-on target has a separately named work unit and step.",
]
out = text
for i, sent in enumerate(boxes):
    pat = re.compile(r"- \[ \] (" + re.escape(sent) + r")( VIOLATION:[^\n]*)?")
    repl = "- [x] " + sent + (violation if i == 2 else "")
    out, n = pat.subn(repl, out, count=1)
    if n == 0:
        print("atomicity: box not found: " + sent, file=sys.stderr)
open(path, "w").write(out)
PYATOMIC
if [ -n "$extra" ]; then
    printf 'atomicity: VIOLATION — also touched: %s\n' "$(printf '%s' "$extra" | tr '\n' ' ')" >&2
else
    printf 'atomicity: diff matches declared target %s; boxes ticked\n' "$declared_target" >&2
fi

