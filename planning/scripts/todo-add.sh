#!/usr/bin/env bash
# MODE: DEV
# todo-add.sh — append one task to TODO.json through the shared register
# checks, then resort. The register is the queue; nothing writes it by hand.
#
# Usage:
#   todo-add.sh --id T45 --title "text" [--parent T44] [--priority high]
#               [--status open] [--blocked-on X] [--detail "text"]
#               [--ref path]... 
#   todo-add.sh --help

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/register-lib.sh
source "$script_dir/register-lib.sh"

file="${TODO_JSON:-$(cd "$script_dir/.." && pwd)/TODO.json}"
# --help answers before anything touches the filesystem: a missing register
# is a hard error for writes but must not hide usage.
case "${1:-}" in
    -h|--help) sed -n '3,10p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
esac

[ -f "$file" ] || { printf '%s: register not found: %s\n' "${0##*/}" "$file" >&2; exit 66; }

id="" title="" parent="null" priority="normal" status="open" blocked_on="null" detail="null"
declare -a refs=()
# jq is the ceiling of the required runtime: refuse with 69 rather than
# half-running when it is missing (mirrors validate-plan.sh).
if ! command -v jq >/dev/null 2>&1; then
    printf '%s: jq is required (it reads and writes the JSON registers); install jq and re-run\n' \
        "${0##*/}" >&2
    exit 69
fi

while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) sed -n '2,12p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
        --id) [ "$#" -ge 2 ] || exit 64; id="$2"; shift 2 ;;
        --title) [ "$#" -ge 2 ] || exit 64; title="$2"; shift 2 ;;
        --parent) [ "$#" -ge 2 ] || exit 64; parent="\"$2\""; shift 2 ;;
        --priority) [ "$#" -ge 2 ] || exit 64; priority="$2"; shift 2 ;;
        --status) [ "$#" -ge 2 ] || exit 64; status="$2"; shift 2 ;;
        --blocked-on) [ "$#" -ge 2 ] || exit 64; blocked_on="\"$2\""; shift 2 ;;
        --detail) [ "$#" -ge 2 ] || exit 64; detail="\"$2\""; shift 2 ;;
        --ref) [ "$#" -ge 2 ] || exit 64; refs+=("$2"); shift 2 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
    esac
done
[ -n "$id" ] && [ -n "$title" ] || { printf '%s: --id and --title are required\n' "${0##*/}" >&2; exit 64; }

# Duplicate ids are refused before anything is written: appending first and
# validating after left a poisoned register behind every refused add.
items_key="tasks"
jq -e --arg id "$id" --arg k "$items_key" \
    '.tasks[] | select(.id == $id)' "$file" >/dev/null 2>&1 && {
    printf '%s: duplicate ids\n' "${0##*/}" >&2
    exit 65
}

now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
refs_json="[]"
if [ "${#refs[@]}" -gt 0 ]; then
    refs_json="$(printf '%s\n' "${refs[@]}" | jq -R . | jq -s .)"
fi

jq --arg id "$id" --arg title "$title" --arg now "$now" \
   --argjson parent "$parent" --arg priority "$priority" --arg status "$status" \
   --argjson blocked_on "$blocked_on" --argjson detail "$detail" --argjson refs "$refs_json" '
   .tasks += [{id: $id, title: $title, status: $status, priority: $priority,
               parent: $parent, detail: (if $detail == "null" then null else $detail end),
               blocked_on: $blocked_on, refs: $refs, note: null,
               created_at: $now, updated_at: $now }]
' "$file" > "$file.tmp"
mv "$file.tmp" "$file"

reg_write todo "$file"
new_id="$(reg_next_id todo "$file")"
printf 'Added %s (next free: T%d)\n' "$id" "$((new_id - 1))"
