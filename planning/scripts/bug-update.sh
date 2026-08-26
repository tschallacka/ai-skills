#!/usr/bin/env bash
# MODE: DEV
# bug-update.sh — set one defect's status (closing it), priority, or append to
# its mechanism/verification/notes, through the shared register checks.
#
# Usage:
#   bug-update.sh <id> --status fixed --fix "commit — what changed" \
#                      --verification "how proven, including the mutation"
#   bug-update.sh <id> --status wont-fix --reason "why, for the record"
#   bug-update.sh <id> --priority high | --note "extra finding"
#   bug-update.sh <id> --mechanism "root cause" | --reason "why" \
#                      | --append-note "appended"
#   bug-update.sh --help
#
# Closing as fixed requires --fix and --verification; wont-fix/not-a-defect/
# obsolete require --reason. A status change without its evidence is refused.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/register-lib.sh
source "$script_dir/register-lib.sh"

file="${BUGS_JSON:-$(cd "$script_dir/.." && pwd)/BUGS.json}"
# --help answers before anything touches the filesystem: a missing register
# is a hard error for writes but must not hide usage.
case "${1:-}" in
    -h|--help) sed -n '3,13p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
esac

[ -f "$file" ] || { printf '%s: register not found: %s\n' "${0##*/}" "$file" >&2; exit 66; }

id="${1:-}"; [ -n "$id" ] && shift || {
    printf 'usage: %s <id> [--status S] [--fix F] [--verification V] [--reason R] [--priority P] [--mechanism M] [--append-note N]\n' "${0##*/}" >&2
    exit 64
}

status_val="" fix_val="" ver_val="" reason_val="" pri_val="" mech_val="" note_val=""
# jq is the ceiling of the required runtime: refuse with 69 rather than
# half-running when it is missing (mirrors validate-plan.sh).
if ! command -v jq >/dev/null 2>&1; then
    printf '%s: jq is required (it reads and writes the JSON registers); install jq and re-run\n' \
        "${0##*/}" >&2
    exit 69
fi

while [ "$#" -gt 0 ]; do
    case "$1" in
        --status) [ "$#" -ge 2 ] || exit 64; status_val="$2"; shift 2 ;;
        --fix) [ "$#" -ge 2 ] || exit 64; fix_val="$2"; shift 2 ;;
        --verification) [ "$#" -ge 2 ] || exit 64; ver_val="$2"; shift 2 ;;
        --reason) [ "$#" -ge 2 ] || exit 64; reason_val="$2"; shift 2 ;;
        --priority) [ "$#" -ge 2 ] || exit 64; pri_val="$2"; shift 2 ;;
        --mechanism) [ "$#" -ge 2 ] || exit 64; mech_val="$2"; shift 2 ;;
        --append-note) [ "$#" -ge 2 ] || exit 64; note_val="$2"; shift 2 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
    esac
done

[ -n "$status_val$fix_val$ver_val$reason_val$pri_val$mech_val$note_val" ] \
    || { printf '%s: nothing to set\n' "${0##*/}" >&2; exit 64; }

jq -e --arg id "$id" '.bugs[]? | select(.id == $id)' "$file" >/dev/null \
    || { printf '%s: no defect %s\n' "${0##*/}" "$id" >&2; exit 66; }

# Closing evidence rules: fixed needs what changed AND how it is proven;
# wont-fix / not-a-defect / obsolete need the reasoning in --reason.
case "$status_val" in
    fixed)
        [ -n "$fix_val" ] || { printf '%s: --status fixed requires --fix\n' "${0##*/}" >&2; exit 64; }
        [ -n "$ver_val" ] || { printf '%s: --status fixed requires --verification\n' "${0##*/}" >&2; exit 64; }
        ;;
    wont-fix|not-a-defect|obsolete)
        [ -n "$reason_val" ] || { printf '%s: --status %s requires --reason\n' "${0##*/}" "$status_val" >&2; exit 64; }
        ;;
esac

now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
tmp="$(mktemp "${TMPDIR:-/tmp}/bug-update.XXXXXX")"
jq --arg id "$id" --arg now "$now" \
   --arg status "$status_val" --arg fix "$fix_val" --arg ver "$ver_val" \
   --arg reason "$reason_val" --arg pri "$pri_val" --arg mech "$mech_val" --arg note "$note_val" '
   .bugs |= map(
       if .id == $id then
         .updated_at = $now
         | (if $status != "" then .status = $status else . end)
         | (if $pri != "" then .priority = $pri else . end)
         | (if $fix != "" then .fix = $fix else . end)
         | (if $ver != "" then .verification = $ver else . end)
         | (if $mech != "" then .mechanism = $mech else . end)
         | (if $reason != "" then .notes = ((.notes // "") + (if (.notes // "") != "" then " " else "" end) + $reason) else . end)
         | (if $note != "" then .notes = ((.notes // "") + (if (.notes // "") != "" then " " else "" end) + $note) else . end)
       else . end)
' "$file" > "$tmp"
mv "$tmp" "$file"

reg_write bug "$file"
printf 'Updated %s\n' "$id"
