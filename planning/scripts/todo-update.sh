#!/usr/bin/env bash
# MODE: DEV
# todo-update.sh — set one task's status, priority, note, detail or
# blocked_on through the shared register checks, then resort.
#
# Usage:
#   todo-update.sh <id> [--status done] [--priority high] [--note "evidence"]
#                   [--detail "..."] [--blocked-on X]
#   todo-update.sh <id> --help
#
# Setting --status done on a task without evidence in --note is refused:
# every closed item carries its verification.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/register-lib.sh
source "$script_dir/register-lib.sh"

file="${TODO_JSON:-$(cd "$script_dir/.." && pwd)/TODO.json}"
[ -f "$file" ] || { printf '%s: register not found: %s\n' "${0##*/}" "$file" >&2; exit 66; }

id="${1:-}"; [ -n "$id" ] && shift || {
    printf 'usage: %s <id> [--status S] [--priority P] [--note N] [--detail D] [--blocked-on X]\n' "${0##*/}" >&2
    exit 64
}

status_val="" pri_val="" note_val="" detail_val="" blocked_val=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --status) [ "$#" -ge 2 ] || exit 64; status_val="$2"; shift 2 ;;
        --priority) [ "$#" -ge 2 ] || exit 64; pri_val="$2"; shift 2 ;;
        --note) [ "$#" -ge 2 ] || exit 64; note_val="$2"; shift 2 ;;
        --detail) [ "$#" -ge 2 ] || exit 64; detail_val="$2"; shift 2 ;;
        --blocked-on) [ "$#" -ge 2 ] || exit 64; blocked_val="$2"; shift 2 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
    esac
done

[ -n "$status_val$pri_val$note_val$detail_val$blocked_val" ] \
    || { printf '%s: nothing to set\n' "${0##*/}" >&2; exit 64; }

jq -e --arg id "$id" '.tasks[]? | select(.id == $id)' "$file" >/dev/null \
    || { printf '%s: no task %s\n' "${0##*/}" "$id" >&2; exit 66; }

now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
tmp="$(mktemp "${TMPDIR:-/tmp}/todo-update.XXXXXX")"
jq --arg id "$id" --arg now "$now" \
   --arg status "$status_val" --arg pri "$pri_val" --arg note "$note_val" \
   --arg detail "$detail_val" --arg blocked "$blocked_val" '
   .tasks |= map(
       if .id == $id then
         .updated_at = $now
         | (if $status != "" then .status = $status else . end)
         | (if $pri != "" then .priority = $pri else . end)
         | (if $note != "" then .note = $note else . end)
         | (if $detail != "" then .detail = $detail else . end)
         | (if $blocked != "" then .blocked_on = $blocked else . end)
       else . end)
' "$file" > "$tmp"

findings="$(reg_findings todo "$tmp")"
if [ -n "$findings" ]; then
    printf '%s\n' "$findings" >&2
    rm -f "$tmp"
    printf '%s: update refused; nothing was written\n' "${0##*/}" >&2
    exit 65
fi
mv "$tmp" "$file"
printf 'Updated %s\n' "$id"
printf 'Updated %s\n' "$id"
