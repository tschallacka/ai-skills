#!/usr/bin/env bash
# MODE: DEV
# bug-add.sh — append one defect to BUGS.json through the shared register
# checks, then resort. Reproduction, observation and expectation are required:
# a report without a reproduction is a rumour.
#
# Usage:
#   bug-add.sh --title "text" --reproduce "cmd" --observed "text" \
#              --expected "text" [--severity major] [--priority normal]
#              [--status reported|confirmed] [--mechanism "text"]
#              [--parent B37] [--found-by "who"] [--surfaces f1,f2]
#   bug-add.sh --help

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/register-lib.sh
source "$script_dir/register-lib.sh"

file="${BUGS_JSON:-$(cd "$script_dir/.." && pwd)/BUGS.json}"
# --help answers before anything touches the filesystem: a missing register
# is a hard error for writes but must not hide usage.
case "${1:-}" in
    -h|--help) sed -n '3,12p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
esac

[ -f "$file" ] || { printf '%s: register not found: %s\n' "${0##*/}" "$file" >&2; exit 66; }

title="" reproduce="" observed="" expected="" severity="major" priority="normal"
status="reported" mechanism="null" parent="null" found_by="register writer" surfaces=""
# rjq is the ceiling of the required runtime: refuse with 69 rather than
# half-running when it is missing (mirrors validate-plan.sh).
if ! command -v rjq >/dev/null 2>&1; then
    printf '%s: rjq is required (it reads and writes the JSON registers); install rjq and re-run\n' \
        "${0##*/}" >&2
    exit 69
fi

while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) sed -n '3,12p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
        --title) [ "$#" -ge 2 ] || exit 64; title="$2"; shift 2 ;;
        --reproduce) [ "$#" -ge 2 ] || exit 64; reproduce="$2"; shift 2 ;;
        --observed) [ "$#" -ge 2 ] || exit 64; observed="$2"; shift 2 ;;
        --expected) [ "$#" -ge 2 ] || exit 64; expected="$2"; shift 2 ;;
        --severity) [ "$#" -ge 2 ] || exit 64; severity="$2"; shift 2 ;;
        --priority) [ "$#" -ge 2 ] || exit 64; priority="$2"; shift 2 ;;
        --status) [ "$#" -ge 2 ] || exit 64; status="$2"; shift 2 ;;
        --mechanism) [ "$#" -ge 2 ] || exit 64; mechanism="$(printf '%s' "$2" | rjq -Rs .)"; shift 2 ;;
        --parent) [ "$#" -ge 2 ] || exit 64; parent="\"$2\""; shift 2 ;;
        --found-by) [ "$#" -ge 2 ] || exit 64; found_by="$2"; shift 2 ;;
        --surfaces) [ "$#" -ge 2 ] || exit 64; surfaces="$2"; shift 2 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
    esac
done
[ -n "$title" ] && [ -n "$reproduce" ] && [ -n "$observed" ] && [ -n "$expected" ] || { printf '%s: --title --reproduce --observed --expected are required\n' "${0##*/}" >&2; exit 64; }


now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
id="B$(reg_next_id bug "$file")"
surfaces_json="[]"
if [ -n "$surfaces" ]; then
    surfaces_json="$(printf '%s' "$surfaces" | tr ',' '\n' | rjq -R . | rjq -s .)"
fi

rjq --arg id "$id" --arg title "$title" --arg now "$now" \
   --arg reproduce "$reproduce" --arg observed "$observed" --arg expected "$expected" \
   --arg severity "$severity" --arg priority "$priority" --arg status "$status" \
   --argjson mechanism "$mechanism" --argjson parent "$parent" \
   --arg found_by "$found_by" --argjson surfaces "$surfaces_json" '
   .bugs += [{
     id: $id, title: $title, status: $status, severity: $severity,
     priority: $priority, parent: $parent,
     reproduce: $reproduce, observed: $observed, expected: $expected,
     mechanism: (if $mechanism == null then null else $mechanism end),
     surfaces: $surfaces, fix: null, verification: null,
     found_by: $found_by, notes: null,
     created_at: $now, updated_at: $now }]
' "$file" > "$file.tmp"

# Validate the RESULT before it lands: an entry that makes the register
# unsound must never reach the file, even behind a later refusal.
findings="$(reg_findings bug "$file.tmp")"
if [ -n "$findings" ]; then
    printf '%s\n' "$findings" >&2
    rm -f "$file.tmp"
    printf '%s: entry refused; nothing was written\n' "${0##*/}" >&2
    exit 65
fi
mv "$file.tmp" "$file"
printf 'Filed %s: %s\n' "$id" "$title"
