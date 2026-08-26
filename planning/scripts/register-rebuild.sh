#!/usr/bin/env bash
# MODE: DEV
# register-rebuild.sh — repair a damaged register mechanically: stamp missing
# timestamps, drop nothing, reorder worst-first, and report every change.
# Damage a stamp cannot fix (unknown statuses, duplicate ids, a fixed entry
# without verification) is refused with the finding list instead.
#
# Usage:
#   register-rebuild.sh bugs [file]     # default BUGS.json at the repo root
#   register-rebuild.sh todo [file]     # default TODO.json at the repo root
#   register-rebuild.sh --help

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/register-lib.sh
source "$script_dir/register-lib.sh"

# jq is the ceiling of the required runtime: refuse with 69 rather than
# half-emitting when it is missing (mirrors validate-plan.sh).
if ! command -v jq >/dev/null 2>&1; then
    printf '%s: jq is required (it assembles the JSON state); install jq and re-run\n' \
        "${0##*/}" >&2
    exit 69
fi

root="$(cd "$script_dir/.." && pwd)"
kind="${1:-}"
case "$kind" in
    bugs) file="${2:-$root/BUGS.json}" ;;
    todo) file="${2:-$root/TODO.json}" ;;
    -h|--help) sed -n '2,12p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) printf 'usage: %s bugs|todo [file]\n' "${0##*/}" >&2; exit 64 ;;
esac
[ -f "$file" ] || { printf '%s: register not found: %s\n' "${0##*/}" "$file" >&2; exit 66; }

now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
arr="$( [ "$kind" = bugs ] && printf bugs || printf tasks )"

tmp="$(mktemp "${TMPDIR:-/tmp}/register-rebuild.XXXXXX")"
if [ "$kind" = bugs ]; then
    jq --arg now "$now" '.bugs |= map(
        .created_at = ((.created_at // "") | if . == "" then $now else . end)
        | .updated_at = ((.updated_at // "") | if . == "" then $now else . end)
        | .severity = ((.severity // "") | if . == "" then "major" else . end)
        | .priority = ((.priority // "") | if . == "" then "normal" else . end)
    )' "$file" > "$tmp"
else
    jq --arg now "$now" '.tasks |= map(
        .created_at = ((.created_at // "") | if . == "" then $now else . end)
        | .updated_at = ((.updated_at // "") | if . == "" then $now else . end)
        | .status = ((.status // "") | if . == "" then "open" else . end)
        | .priority = ((.priority // "") | if . == "" then "normal" else . end)
    )' "$file" > "$tmp"
fi
mv "$tmp" "$file"

reg_sort "${kind%s}" "$file"

findings="$(reg_findings "${kind%s}" "$file")"
if [ -n "$findings" ]; then
    printf '%s\n' "$findings" >&2
    printf '%s: %s still unsound after rebuild — these need human decisions, not stamps\n' \
        "${0##*/}" "$file" >&2
    exit 65
fi

if [ "$kind" = bugs ]; then
    jq --arg now "$now" '.skill_version = ((.skill_version // "") | if . == "" then "1.4.2" else . end)' \
        "$file" > "$tmp" && mv "$tmp" "$file"
fi

printf 'rebuilt %s: stamped, sorted, and sound\n' "$file"
